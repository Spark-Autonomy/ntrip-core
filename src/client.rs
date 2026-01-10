//! NTRIP HTTP client for connecting to casters and streaming corrections.
//!
//! Supports both NTRIP v1 (legacy ICY protocol) and NTRIP v2 (HTTP/1.1 with
//! chunked transfer encoding).
//!
//! Features:
//! - Raw TCP socket connection (plain or TLS-encrypted)
//! - NTRIP v1: HTTP/1.0, ICY 200 OK response
//! - NTRIP v2: HTTP/1.1, chunked transfer encoding
//! - Auto-detection of protocol version from server response
//! - Basic Authentication
//! - TLS/HTTPS support for secure connections
//! - HTTP proxy support via CONNECT tunneling
//! - GGA position reporting

use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

use crate::config::{NtripConfig, NtripVersion, ProxyConfig};
use crate::gga::GgaSentence;
use crate::sourcetable::Sourcetable;
use crate::stream::NtripStream;
use crate::Error;

/// User-Agent string sent in HTTP requests.
const USER_AGENT: &str = concat!("NTRIP ntrip-core/", env!("CARGO_PKG_VERSION"));

/// Detected protocol version after connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DetectedProtocol {
    /// NTRIP v1 - raw binary stream
    V1,
    /// NTRIP v2 - chunked transfer encoding
    V2Chunked,
}

/// NTRIP client for streaming corrections from a caster.
/// Supports both v1 (ICY) and v2 (HTTP/1.1) protocols.
pub struct NtripClient {
    /// Configuration
    config: NtripConfig,
    /// Active stream (plain TCP or TLS, if connected)
    stream: Option<NtripStream>,
    /// Detected protocol after connection
    protocol: Option<DetectedProtocol>,
    /// Buffer for raw over-read bytes from header parsing (read before socket)
    stream_buffer: Vec<u8>,
    /// Buffer for decoded chunk data that exceeded caller's buffer size (v2 chunked mode)
    chunk_buffer: Vec<u8>,
    /// Remaining bytes in current chunk (for v2 chunked mode)
    chunk_remaining: usize,
    /// Last GGA sent (for automatic resend after reconnection)
    last_gga: Option<GgaSentence>,
}

impl NtripClient {
    /// Create a new NTRIP client with the given configuration.
    pub fn new(config: NtripConfig) -> Result<Self, Error> {
        config.validate()?;
        Ok(Self {
            config,
            stream: None,
            protocol: None,
            stream_buffer: Vec::new(),
            chunk_buffer: Vec::new(),
            chunk_remaining: 0,
            last_gga: None,
        })
    }

    /// Connect to the NTRIP caster and start streaming.
    ///
    /// For NTRIP v2, consider using `connect_with_gga()` to send an initial
    /// position in the request headers for faster VRS/nearest-base selection.
    pub async fn connect(&mut self) -> Result<(), Error> {
        self.connect_with_gga(None).await
    }

    /// Establish a TCP connection, optionally through an HTTP proxy.
    ///
    /// If a proxy is configured, this connects to the proxy and uses HTTP CONNECT
    /// to establish a tunnel to the target host:port.
    async fn establish_tcp_connection(
        target_host: &str,
        target_port: u16,
        proxy: Option<&ProxyConfig>,
        timeout_secs: u32,
    ) -> Result<TcpStream, Error> {
        let timeout = Duration::from_secs(timeout_secs as u64);

        match proxy {
            Some(proxy_config) => {
                let proxy_addr = format!("{}:{}", proxy_config.host, proxy_config.port);
                info!(
                    proxy = %proxy_addr,
                    target = %format!("{}:{}", target_host, target_port),
                    "Connecting via HTTP proxy"
                );

                // Connect to proxy
                let mut stream =
                    match tokio::time::timeout(timeout, TcpStream::connect(&proxy_addr)).await {
                        Ok(Ok(s)) => s,
                        Ok(Err(e)) => {
                            return Err(Error::ProxyError {
                                message: format!(
                                    "Failed to connect to proxy {}:{}: {}",
                                    proxy_config.host, proxy_config.port, e
                                ),
                            })
                        }
                        Err(_) => return Err(Error::Timeout { timeout_secs }),
                    };

                // Build HTTP CONNECT request
                let mut connect_request = format!(
                    "CONNECT {}:{} HTTP/1.1\r\nHost: {}:{}\r\n",
                    target_host, target_port, target_host, target_port
                );

                // Add proxy authentication if configured
                if let (Some(user), Some(pass)) = (&proxy_config.username, &proxy_config.password) {
                    let credentials = format!("{}:{}", user, pass);
                    let encoded = BASE64.encode(credentials);
                    connect_request
                        .push_str(&format!("Proxy-Authorization: Basic {}\r\n", encoded));
                }

                connect_request.push_str("\r\n");

                debug!("Sending HTTP CONNECT request to proxy");

                // Send CONNECT request
                stream
                    .write_all(connect_request.as_bytes())
                    .await
                    .map_err(|e| Error::ProxyError {
                        message: format!("Failed to send CONNECT request: {}", e),
                    })?;

                // Read response (first line should be "HTTP/1.x 200 ...")
                let mut reader = BufReader::new(&mut stream);
                let mut response_line = String::new();

                match tokio::time::timeout(timeout, reader.read_line(&mut response_line)).await {
                    Ok(Ok(0)) => {
                        return Err(Error::ProxyError {
                            message: "Proxy closed connection without response".to_string(),
                        })
                    }
                    Ok(Ok(_)) => {}
                    Ok(Err(e)) => {
                        return Err(Error::ProxyError {
                            message: format!("Failed to read proxy response: {}", e),
                        })
                    }
                    Err(_) => return Err(Error::Timeout { timeout_secs }),
                }

                // Check for 200 response
                if !response_line.contains("200") {
                    return Err(Error::ProxyError {
                        message: format!("Proxy CONNECT failed: {}", response_line.trim()),
                    });
                }

                debug!(response = %response_line.trim(), "Proxy tunnel established");

                // Consume remaining headers until empty line
                loop {
                    let mut header_line = String::new();
                    match tokio::time::timeout(timeout, reader.read_line(&mut header_line)).await {
                        Ok(Ok(0)) => break,
                        Ok(Ok(_)) => {
                            if header_line.trim().is_empty() {
                                break;
                            }
                        }
                        Ok(Err(e)) => {
                            return Err(Error::ProxyError {
                                message: format!("Failed to read proxy headers: {}", e),
                            })
                        }
                        Err(_) => return Err(Error::Timeout { timeout_secs }),
                    }
                }

                info!("HTTP proxy tunnel established successfully");
                Ok(stream)
            }
            None => {
                // Direct connection (no proxy)
                let addr = format!("{}:{}", target_host, target_port);
                match tokio::time::timeout(timeout, TcpStream::connect(&addr)).await {
                    Ok(Ok(s)) => Ok(s),
                    Ok(Err(e)) => Err(Error::connection_failed(target_host, target_port, e)),
                    Err(_) => Err(Error::Timeout { timeout_secs }),
                }
            }
        }
    }

    /// Connect to the NTRIP caster with an optional initial GGA position.
    ///
    /// For NTRIP v2, the GGA sentence is sent in the `Ntrip-GGA` header,
    /// allowing the caster to select the appropriate VRS or nearest base
    /// immediately, without waiting for a post-connection GGA report.
    ///
    /// For NTRIP v1, the initial_gga is ignored (GGA must be sent post-connection).
    pub async fn connect_with_gga(
        &mut self,
        initial_gga: Option<&GgaSentence>,
    ) -> Result<(), Error> {
        let host = &self.config.host;
        let port = self.config.port;
        let addr = format!("{}:{}", host, port);

        info!(addr = %addr, "Connecting to NTRIP caster");

        // 1. Establish TCP connection (directly or via proxy)
        let tcp_stream = Self::establish_tcp_connection(
            host,
            port,
            self.config.proxy.as_ref(),
            self.config.connection.timeout_secs,
        )
        .await?;

        // 2. Optionally upgrade to TLS
        let mut stream: NtripStream = if self.config.use_tls {
            if self.config.tls_skip_verify {
                warn!("TLS certificate verification is disabled - connection is not fully secure");
            }
            NtripStream::connect_tls(tcp_stream, host, self.config.tls_skip_verify).await?
        } else {
            NtripStream::plain(tcp_stream)
        };

        // 3. Construct HTTP Request based on configured version
        let mountpoint = &self.config.mountpoint;

        let auth_header =
            if let (Some(user), Some(pass)) = (&self.config.username, &self.config.password) {
                let credentials = format!("{}:{}", user, pass);
                let encoded = BASE64.encode(credentials);
                format!("Authorization: Basic {}\r\n", encoded)
            } else {
                String::new()
            };

        // Build Ntrip-GGA header for v2 if initial position provided
        let gga_header = match (&self.config.ntrip_version, initial_gga) {
            (NtripVersion::V2, Some(gga)) | (NtripVersion::Auto, Some(gga)) => {
                let nmea = gga.to_nmea();
                let nmea_trimmed = nmea.trim();
                debug!(gga = %nmea_trimmed, "Including initial GGA in request header");
                format!("Ntrip-GGA: {}\r\n", nmea_trimmed)
            }
            _ => String::new(),
        };

        // Store initial GGA for potential reconnection resend (fulfills documented behavior)
        if let Some(gga) = initial_gga {
            self.last_gga = Some(gga.clone());
        }

        // Build request based on configured NTRIP version
        let request = match self.config.ntrip_version {
            NtripVersion::V1 => {
                debug!("Using NTRIP v1 protocol");
                format!(
                    "GET /{} HTTP/1.0\r\n\
                     User-Agent: {}\r\n\
                     Host: {}:{}\r\n\
                     Accept: */*\r\n\
                     Connection: close\r\n\
                     {}\r\n",
                    mountpoint, USER_AGENT, host, port, auth_header
                )
            }
            NtripVersion::V2 | NtripVersion::Auto => {
                debug!("Using NTRIP v2 protocol request");
                format!(
                    "GET /{} HTTP/1.1\r\n\
                     Host: {}:{}\r\n\
                     User-Agent: {}\r\n\
                     Ntrip-Version: Ntrip/2.0\r\n\
                     {}\
                     Accept: */*\r\n\
                     Connection: close\r\n\
                     {}\r\n",
                    mountpoint, host, port, USER_AGENT, gga_header, auth_header
                )
            }
        };

        // Log request with Authorization header redacted for security
        let redacted_request = if auth_header.is_empty() {
            request.clone()
        } else {
            request.replace(&auth_header, "Authorization: Basic [REDACTED]\r\n")
        };
        debug!(request = %redacted_request, "Sending NTRIP request");

        // 4. Send Request
        if let Err(e) = stream.write_all(request.as_bytes()).await {
            return Err(Error::NetworkError { source: e });
        }

        // 5. Read Response Headers (buffered for efficiency)
        const MAX_HEADER_SIZE: usize = 4096;
        let mut headers = Vec::with_capacity(512);
        let mut buffer = [0u8; 512];

        let header_end_found = loop {
            if headers.len() >= MAX_HEADER_SIZE {
                break false;
            }

            // Read a chunk of data
            let bytes_to_read = std::cmp::min(buffer.len(), MAX_HEADER_SIZE - headers.len());
            match stream.read(&mut buffer[..bytes_to_read]).await {
                Ok(0) => break false, // Connection closed
                Ok(n) => {
                    headers.extend_from_slice(&buffer[..n]);

                    // Check for standard HTTP header end
                    if headers.len() >= 4 {
                        if let Some(pos) = headers.windows(4).position(|w| w == b"\r\n\r\n") {
                            // Preserve any over-read bytes (data after headers) in stream_buffer
                            let body_start = pos + 4;
                            if headers.len() > body_start {
                                self.stream_buffer.extend_from_slice(&headers[body_start..]);
                            }
                            // Trim headers to just before the body
                            headers.truncate(body_start);
                            break true;
                        }
                    }

                    // Check for legacy ICY response (NTRIP v1)
                    if headers.len() >= 12 && headers.starts_with(b"ICY 200 OK\r\n") {
                        debug!("Detected legacy ICY 200 OK response, starting stream");
                        // For ICY, preserve any bytes after "ICY 200 OK\r\n" in stream_buffer
                        let icy_header_len = 12; // b"ICY 200 OK\r\n".len()
                        if headers.len() > icy_header_len {
                            self.stream_buffer
                                .extend_from_slice(&headers[icy_header_len..]);
                        }
                        headers.truncate(icy_header_len);
                        break true;
                    }
                }
                Err(e) => return Err(Error::NetworkError { source: e }),
            }
        };

        if !header_end_found {
            return Err(Error::NetworkError {
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Header too large or incomplete",
                ),
            });
        }

        // 6. Parse Status Line and detect protocol version
        let response = String::from_utf8_lossy(&headers);
        let status_line = response.lines().next().unwrap_or("");

        debug!(status_line = %status_line, "Received NTRIP response");

        if status_line.starts_with("ICY 200") || status_line.contains("200 OK") {
            let detected = self.detect_protocol(&response, status_line);
            info!(
                mountpoint = %mountpoint,
                protocol = ?detected,
                "Connected to NTRIP caster"
            );

            self.stream = Some(stream);
            self.protocol = Some(detected);
            // Note: chunk_buffer may contain over-read bytes from header parsing - do not clear
            self.chunk_remaining = 0;
            Ok(())
        } else if status_line.contains("401 Unauthorized") {
            error!("NTRIP authentication failed");
            Err(Error::AuthenticationFailed {
                host: host.clone(),
                username: self.config.username.clone().unwrap_or_default(),
            })
        } else if status_line.contains("404 Not Found") {
            error!(mountpoint = %mountpoint, "Mountpoint not found");
            Err(Error::MountpointNotFound {
                host: host.clone(),
                mountpoint: mountpoint.clone(),
            })
        } else {
            error!(status = %status_line, "NTRIP caster error");
            Err(Error::HttpError {
                host: host.clone(),
                status_code: Self::parse_status_code(status_line),
                reason: status_line.to_string(),
            })
        }
    }

    /// Read a chunk of RTCM data from the stream with timeout.
    /// Automatically handles chunked transfer encoding for NTRIP v2.
    ///
    /// # Reconnection Behavior
    ///
    /// If automatic reconnection is enabled (default: 3 attempts), this method
    /// will attempt to reconnect on timeout or disconnect errors before returning
    /// an error to the caller.
    ///
    /// **Important:** When reconnection is enabled, this method may block for up to
    /// `(max_reconnect_attempts × reconnect_delay_ms) + connection_timeout` milliseconds
    /// while attempting to restore the connection. During this time:
    /// - The original error (timeout/disconnect) is not immediately returned
    /// - If a previous GGA position was set (via `send_gga()` or `connect_with_gga()`),
    ///   it will be automatically resent after successful reconnection
    ///
    /// To disable this behavior, use `config.without_reconnect()`.
    pub async fn read_chunk(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let max_attempts = self.config.connection.max_reconnect_attempts;
        let reconnect_delay = self.config.connection.reconnect_delay_ms;
        let mut attempts = 0;

        loop {
            // Ensure we're connected
            if self.stream.is_none() {
                if attempts > 0 && attempts <= max_attempts {
                    // Attempting reconnection
                    warn!(
                        attempt = attempts,
                        max_attempts = max_attempts,
                        "Attempting automatic reconnection"
                    );

                    // Delay before reconnect
                    tokio::time::sleep(Duration::from_millis(reconnect_delay)).await;

                    // Clone GGA before mutable borrow for reconnection
                    let gga_clone = self.last_gga.clone();
                    match self.connect_with_gga(gga_clone.as_ref()).await {
                        Ok(()) => {
                            info!(attempt = attempts, "Reconnection successful");
                            // Resend last GGA if we have one (for v1 protocol)
                            if let Some(ref gga) = gga_clone {
                                if let Err(e) = self.send_gga_internal(gga).await {
                                    warn!(error = %e, "Failed to resend GGA after reconnect");
                                }
                            }
                        }
                        Err(e) => {
                            warn!(attempt = attempts, error = %e, "Reconnection attempt failed");
                            attempts += 1;
                            continue;
                        }
                    }
                } else if attempts > max_attempts {
                    // Exhausted all reconnection attempts
                    return Err(Error::StreamDisconnected {
                        reason: format!(
                            "Connection lost after {} reconnection attempts",
                            max_attempts
                        ),
                    });
                } else {
                    // First attempt, not connected yet
                    return Err(Error::StreamDisconnected {
                        reason: "Not connected".to_string(),
                    });
                }
            }

            let read_timeout_secs = self.config.connection.read_timeout_secs;

            // If read_timeout_secs == 0, bypass timeout (as documented: 0 = no timeout)
            let read_result = if read_timeout_secs == 0 {
                self.read_raw_or_chunked(buf).await.map(Some)
            } else {
                let timeout_duration = Duration::from_secs(read_timeout_secs as u64);
                match tokio::time::timeout(timeout_duration, self.read_raw_or_chunked(buf)).await {
                    Ok(result) => result.map(Some),
                    Err(_) => Ok(None), // Timeout occurred
                }
            };

            match read_result {
                Ok(Some(n)) => {
                    debug!(bytes = n, "Received data");
                    return Ok(n);
                }
                Ok(None) => {
                    // Timeout - potentially recoverable
                    self.stream = None;
                    self.reset_chunk_state();
                    if max_attempts > 0 {
                        attempts += 1;
                        continue;
                    }
                    return Err(Error::ReadTimeout {
                        timeout_secs: read_timeout_secs,
                    });
                }
                Err(e) => {
                    // Check if this is a recoverable error
                    if max_attempts > 0 && Self::is_recoverable_error(&e) {
                        attempts += 1;
                        self.stream = None;
                        self.reset_chunk_state();
                        continue;
                    }
                    return Err(e);
                }
            }
        }
    }

    /// Check if an error is potentially recoverable via reconnection.
    fn is_recoverable_error(error: &Error) -> bool {
        matches!(
            error,
            Error::StreamDisconnected { .. }
                | Error::NetworkError { .. }
                | Error::ReadTimeout { .. }
        )
    }

    /// Reset chunked decoding state (used after reconnection).
    fn reset_chunk_state(&mut self) {
        self.stream_buffer.clear();
        self.chunk_buffer.clear();
        self.chunk_remaining = 0;
    }

    /// Internal GGA send that doesn't update last_gga (used for reconnection resend).
    async fn send_gga_internal(&mut self, gga: &GgaSentence) -> Result<(), Error> {
        let stream = self.stream.as_mut().ok_or(Error::StreamDisconnected {
            reason: "Not connected".into(),
        })?;

        let nmea = gga.to_nmea();
        debug!(gga = %nmea.trim(), "Resending GGA after reconnection");

        stream
            .write_all(nmea.as_bytes())
            .await
            .map_err(|e| Error::NetworkError { source: e })?;

        Ok(())
    }

    /// Check if connected.
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Disconnect from the caster.
    pub fn disconnect(&mut self) {
        if self.stream.take().is_some() {
            debug!("Disconnected from NTRIP caster");
        }
    }

    /// Send a GGA position report to the caster on the existing stream.
    pub async fn send_gga(&mut self, gga: &GgaSentence) -> Result<(), Error> {
        let stream = self.stream.as_mut().ok_or(Error::StreamDisconnected {
            reason: "Not connected".into(),
        })?;

        let nmea = gga.to_nmea();
        debug!(gga = %nmea.trim(), "Sending GGA to caster");

        stream
            .write_all(nmea.as_bytes())
            .await
            .map_err(|e| Error::NetworkError { source: e })?;

        // Store for potential reconnection
        self.last_gga = Some(gga.clone());

        Ok(())
    }

    /// Get the configuration.
    pub fn config(&self) -> &NtripConfig {
        &self.config
    }

    /// Fetch the sourcetable from the NTRIP caster.
    ///
    /// This is a one-shot operation that connects, retrieves the sourcetable,
    /// and disconnects. It does not affect the current streaming connection.
    pub async fn get_sourcetable(config: &NtripConfig) -> Result<Sourcetable, Error> {
        config.validate()?;
        let host = &config.host;
        let port = config.port;
        let addr = format!("{}:{}", host, port);

        info!(addr = %addr, "Fetching sourcetable from NTRIP caster");

        // 1. Establish TCP connection (directly or via proxy)
        let tcp_stream = Self::establish_tcp_connection(
            host,
            port,
            config.proxy.as_ref(),
            config.connection.timeout_secs,
        )
        .await?;

        // 2. Optionally upgrade to TLS
        let mut stream: NtripStream = if config.use_tls {
            NtripStream::connect_tls(tcp_stream, host, config.tls_skip_verify).await?
        } else {
            NtripStream::plain(tcp_stream)
        };

        // 3. Send sourcetable request
        let auth_header = if let (Some(user), Some(pass)) = (&config.username, &config.password) {
            let credentials = format!("{}:{}", user, pass);
            let encoded = BASE64.encode(credentials);
            format!("Authorization: Basic {}\r\n", encoded)
        } else {
            String::new()
        };

        let request = format!(
            "GET / HTTP/1.0\r\n\
             User-Agent: {}\r\n\
             Host: {}:{}\r\n\
             Accept: */*\r\n\
             Connection: close\r\n\
             {}\r\n",
            USER_AGENT, host, port, auth_header
        );

        // Log request with Authorization header redacted for security
        let redacted_request = if auth_header.is_empty() {
            request.clone()
        } else {
            request.replace(&auth_header, "Authorization: Basic [REDACTED]\r\n")
        };
        debug!(request = %redacted_request, "Sending sourcetable request");

        if let Err(e) = stream.write_all(request.as_bytes()).await {
            return Err(Error::NetworkError { source: e });
        }

        // 4. Read entire response
        let mut response = Vec::new();
        let mut buf = [0u8; 4096];

        loop {
            match tokio::time::timeout(
                Duration::from_secs(config.connection.timeout_secs as u64),
                stream.read(&mut buf),
            )
            .await
            {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => response.extend_from_slice(&buf[..n]),
                Ok(Err(e)) => return Err(Error::NetworkError { source: e }),
                Err(_) => {
                    return Err(Error::Timeout {
                        timeout_secs: config.connection.timeout_secs,
                    })
                }
            }

            if response.len() > 1_000_000 {
                return Err(Error::SourcetableParseError {
                    message: "Sourcetable too large (>1MB)".to_string(),
                });
            }
        }

        // 5. Parse response
        let response_str = String::from_utf8_lossy(&response);

        let first_line = response_str.lines().next().unwrap_or("");
        if !first_line.contains("200") {
            return Err(Error::HttpError {
                host: host.clone(),
                status_code: Self::parse_status_code(first_line),
                reason: first_line.to_string(),
            });
        }

        let body = if let Some(idx) = response_str.find("\r\n\r\n") {
            &response_str[idx + 4..]
        } else {
            &response_str[..]
        };

        let table = Sourcetable::parse(body);

        info!(
            streams = table.streams.len(),
            casters = table.casters.len(),
            networks = table.networks.len(),
            "Parsed sourcetable"
        );

        Ok(table)
    }

    /// Parse HTTP status code from status line (e.g., "HTTP/1.1 401 Unauthorized" -> 401).
    fn parse_status_code(status_line: &str) -> u16 {
        status_line
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0)
    }

    /// Detect protocol version from server response headers.
    fn detect_protocol(&self, response: &str, status_line: &str) -> DetectedProtocol {
        if status_line.starts_with("ICY") {
            debug!("Detected NTRIP v1 (ICY response)");
            return DetectedProtocol::V1;
        }

        let response_lower = response.to_lowercase();
        if response_lower.contains("transfer-encoding: chunked") {
            debug!("Detected NTRIP v2 (chunked transfer encoding)");
            return DetectedProtocol::V2Chunked;
        }

        if response_lower.contains("ntrip-version:") {
            debug!("Server indicates NTRIP v2 but response is not chunked; treating as raw stream");
            return DetectedProtocol::V1;
        }

        debug!("Defaulting to NTRIP v1 protocol (raw stream)");
        DetectedProtocol::V1
    }

    /// Read a chunk of data, handling chunked transfer encoding for v2.
    async fn read_raw_or_chunked(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let protocol = self.protocol.unwrap_or(DetectedProtocol::V1);

        match protocol {
            DetectedProtocol::V1 => self.read_raw(buf).await,
            DetectedProtocol::V2Chunked => self.read_chunked(buf).await,
        }
    }

    /// Read raw data from stream (v1 protocol).
    async fn read_raw(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        // First drain any over-read bytes from header parsing
        if !self.stream_buffer.is_empty() {
            let to_copy = std::cmp::min(buf.len(), self.stream_buffer.len());
            buf[..to_copy].copy_from_slice(&self.stream_buffer[..to_copy]);
            self.stream_buffer.drain(..to_copy);
            return Ok(to_copy);
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| Error::StreamDisconnected {
                reason: "Not connected".to_string(),
            })?;

        match stream.read(buf).await {
            Ok(0) => {
                self.stream = None;
                Err(Error::StreamDisconnected {
                    reason: "Server closed connection".to_string(),
                })
            }
            Ok(n) => Ok(n),
            Err(e) => {
                self.stream = None;
                Err(Error::NetworkError { source: e })
            }
        }
    }

    /// Read chunked transfer encoded data (v2 protocol).
    async fn read_chunked(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        // First return any already-decoded chunk data
        if !self.chunk_buffer.is_empty() {
            let to_copy = std::cmp::min(buf.len(), self.chunk_buffer.len());
            buf[..to_copy].copy_from_slice(&self.chunk_buffer[..to_copy]);
            self.chunk_buffer.drain(..to_copy);
            return Ok(to_copy);
        }

        // If we have remaining bytes in current chunk, read them
        if self.chunk_remaining > 0 {
            let to_read = std::cmp::min(buf.len(), self.chunk_remaining);
            let n = self.read_from_stream(&mut buf[..to_read]).await?;
            if n == 0 {
                self.stream = None;
                return Err(Error::StreamDisconnected {
                    reason: "Server closed connection".to_string(),
                });
            }
            self.chunk_remaining -= n;
            if self.chunk_remaining == 0 {
                // Consume trailing CRLF after chunk data
                self.read_and_validate_crlf().await?;
            }
            return Ok(n);
        }

        // Read chunk size line
        let mut size_line = Vec::new();
        loop {
            let byte = self.read_one_byte().await?;
            if byte == b'\n' {
                break;
            }
            if byte != b'\r' {
                size_line.push(byte);
            }
            if size_line.len() > 16 {
                return Err(Error::NetworkError {
                    source: std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Chunk size line too long",
                    ),
                });
            }
        }

        let size_str = String::from_utf8_lossy(&size_line);
        // Handle chunk extensions (RFC 7230): parse only up to ';' and ignore extensions
        let size_part = size_str.trim().split(';').next().unwrap_or("");
        let chunk_size =
            usize::from_str_radix(size_part.trim(), 16).map_err(|_| Error::NetworkError {
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Invalid chunk size: {}", size_str),
                ),
            })?;

        if chunk_size == 0 {
            self.stream = None;
            return Err(Error::StreamDisconnected {
                reason: "Chunked stream ended".to_string(),
            });
        }

        debug!(chunk_size = chunk_size, "Reading chunked data");

        self.chunk_remaining = chunk_size;
        let to_read = std::cmp::min(buf.len(), self.chunk_remaining);

        let n = self.read_from_stream(&mut buf[..to_read]).await?;
        if n == 0 {
            self.stream = None;
            return Err(Error::StreamDisconnected {
                reason: "Server closed connection".to_string(),
            });
        }
        self.chunk_remaining -= n;
        if self.chunk_remaining == 0 {
            // Consume trailing CRLF after chunk data
            self.read_and_validate_crlf().await?;
        }
        Ok(n)
    }

    /// Read bytes from stream_buffer first, then from actual stream.
    async fn read_from_stream(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        // First drain from stream_buffer (over-read bytes from header parsing)
        if !self.stream_buffer.is_empty() {
            let to_copy = std::cmp::min(buf.len(), self.stream_buffer.len());
            buf[..to_copy].copy_from_slice(&self.stream_buffer[..to_copy]);
            self.stream_buffer.drain(..to_copy);
            return Ok(to_copy);
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| Error::StreamDisconnected {
                reason: "Not connected".to_string(),
            })?;

        stream.read(buf).await.map_err(|e| {
            self.stream = None;
            Error::NetworkError { source: e }
        })
    }

    /// Read exactly one byte from stream_buffer or stream.
    async fn read_one_byte(&mut self) -> Result<u8, Error> {
        // First drain from stream_buffer
        if !self.stream_buffer.is_empty() {
            return Ok(self.stream_buffer.remove(0));
        }

        let stream = self
            .stream
            .as_mut()
            .ok_or_else(|| Error::StreamDisconnected {
                reason: "Not connected".to_string(),
            })?;

        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).await.map_err(|e| {
            self.stream = None;
            Error::NetworkError { source: e }
        })?;
        Ok(byte[0])
    }

    /// Read and validate CRLF after chunk data.
    async fn read_and_validate_crlf(&mut self) -> Result<(), Error> {
        let cr = self.read_one_byte().await?;
        let lf = self.read_one_byte().await?;
        if cr != b'\r' || lf != b'\n' {
            self.stream = None;
            return Err(Error::NetworkError {
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Expected CRLF after chunk data",
                ),
            });
        }
        Ok(())
    }
}

impl Drop for NtripClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}
