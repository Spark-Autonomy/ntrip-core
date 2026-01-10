//! Configuration types for ntrip-core.

use crate::Error;
use std::env;
use std::fmt;

/// NTRIP protocol version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum NtripVersion {
    /// NTRIP v1 (HTTP/1.0, ICY 200 OK response)
    V1,
    /// NTRIP v2 (HTTP/1.1, chunked transfer encoding)
    V2,
    /// Auto-detect from server response (default)
    #[default]
    Auto,
}

/// HTTP proxy configuration.
#[derive(Clone)]
pub struct ProxyConfig {
    /// Proxy hostname or IP address.
    pub host: String,
    /// Proxy port.
    pub port: u16,
    /// Username for proxy authentication (optional).
    pub username: Option<String>,
    /// Password for proxy authentication (optional).
    pub password: Option<String>,
}

impl ProxyConfig {
    /// Create a new proxy configuration.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            username: None,
            password: None,
        }
    }

    /// Set proxy authentication credentials.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Parse proxy configuration from a URL string (e.g., "http://user:pass@host:port").
    pub fn from_url(url: &str) -> Option<Self> {
        // Strip http:// or https:// prefix
        let url = url
            .strip_prefix("http://")
            .or_else(|| url.strip_prefix("https://"))
            .unwrap_or(url);

        // Check for credentials (user:pass@host:port)
        let (auth, host_port) = if let Some(at_pos) = url.rfind('@') {
            (Some(&url[..at_pos]), &url[at_pos + 1..])
        } else {
            (None, url)
        };

        // Parse host:port
        let (host, port) = if let Some(colon_pos) = host_port.rfind(':') {
            let port_str = &host_port[colon_pos + 1..];
            let port: u16 = port_str.parse().ok()?;
            (&host_port[..colon_pos], port)
        } else {
            (host_port, 8080) // Default proxy port
        };

        if host.is_empty() {
            return None;
        }

        let mut config = ProxyConfig::new(host, port);

        // Parse credentials if present
        if let Some(auth) = auth {
            if let Some(colon_pos) = auth.find(':') {
                let username = &auth[..colon_pos];
                let password = &auth[colon_pos + 1..];
                config = config.with_credentials(username, password);
            }
        }

        Some(config)
    }

    /// Read proxy configuration from environment variables.
    ///
    /// Checks `$HTTP_PROXY` and `$http_proxy` in that order.
    /// Returns `None` if no proxy is configured or the URL is invalid.
    pub fn from_env() -> Option<Self> {
        env::var("HTTP_PROXY")
            .or_else(|_| env::var("http_proxy"))
            .ok()
            .and_then(|url| Self::from_url(&url))
    }
}

impl fmt::Debug for ProxyConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProxyConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// Connection-related configuration.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Connection timeout in seconds.
    pub timeout_secs: u32,
    /// Read timeout in seconds (0 = no timeout).
    pub read_timeout_secs: u32,
    /// Maximum reconnection attempts on disconnect/timeout (0 = disabled).
    pub max_reconnect_attempts: u32,
    /// Delay between reconnection attempts in milliseconds.
    pub reconnect_delay_ms: u64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 15,
            read_timeout_secs: 30,
            max_reconnect_attempts: 3,
            reconnect_delay_ms: 1000,
        }
    }
}

/// Configuration for an NTRIP client connection.
#[derive(Clone)]
pub struct NtripConfig {
    /// Caster hostname or IP address.
    pub host: String,
    /// Caster port (typically 2101).
    pub port: u16,
    /// Mountpoint name.
    pub mountpoint: String,
    /// Username for authentication.
    pub username: Option<String>,
    /// Password for authentication.
    pub password: Option<String>,
    /// Use HTTPS/TLS.
    pub use_tls: bool,
    /// Skip TLS certificate verification (insecure, for testing only).
    pub tls_skip_verify: bool,
    /// NTRIP protocol version.
    pub ntrip_version: NtripVersion,
    /// Connection configuration.
    pub connection: ConnectionConfig,
    /// HTTP proxy configuration (optional).
    pub proxy: Option<ProxyConfig>,
}

impl fmt::Debug for NtripConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NtripConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("mountpoint", &self.mountpoint)
            .field("username", &self.username)
            .field("password", &self.password.as_ref().map(|_| "[REDACTED]"))
            .field("use_tls", &self.use_tls)
            .field("tls_skip_verify", &self.tls_skip_verify)
            .field("ntrip_version", &self.ntrip_version)
            .field("connection", &self.connection)
            .field("proxy", &self.proxy)
            .finish()
    }
}

impl NtripConfig {
    /// Create a new configuration with required fields.
    pub fn new(host: impl Into<String>, port: u16, mountpoint: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port,
            mountpoint: mountpoint.into(),
            username: None,
            password: None,
            use_tls: false,
            tls_skip_verify: false,
            ntrip_version: NtripVersion::Auto,
            connection: ConnectionConfig::default(),
            proxy: None,
        }
    }

    /// Set credentials for authentication.
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Enable TLS/HTTPS.
    pub fn with_tls(mut self) -> Self {
        self.use_tls = true;
        self
    }

    /// Skip TLS certificate verification (insecure).
    pub fn with_tls_skip_verify(mut self) -> Self {
        self.tls_skip_verify = true;
        self
    }

    /// Set NTRIP protocol version.
    pub fn with_version(mut self, version: NtripVersion) -> Self {
        self.ntrip_version = version;
        self
    }

    /// Set connection timeout.
    pub fn with_timeout(mut self, timeout_secs: u32) -> Self {
        self.connection.timeout_secs = timeout_secs;
        self
    }

    /// Set read timeout.
    pub fn with_read_timeout(mut self, read_timeout_secs: u32) -> Self {
        self.connection.read_timeout_secs = read_timeout_secs;
        self
    }

    /// Set maximum reconnection attempts (0 = disabled).
    pub fn with_reconnect(mut self, max_attempts: u32, delay_ms: u64) -> Self {
        self.connection.max_reconnect_attempts = max_attempts;
        self.connection.reconnect_delay_ms = delay_ms;
        self
    }

    /// Disable automatic reconnection.
    pub fn without_reconnect(mut self) -> Self {
        self.connection.max_reconnect_attempts = 0;
        self
    }

    /// Set HTTP proxy configuration.
    ///
    /// When a proxy is configured, the client will connect to the proxy server
    /// and use HTTP CONNECT to tunnel the connection to the NTRIP caster.
    pub fn with_proxy(mut self, proxy: ProxyConfig) -> Self {
        self.proxy = Some(proxy);
        self
    }

    /// Configure proxy from environment variables.
    ///
    /// Reads `$HTTP_PROXY` or `$http_proxy` environment variable.
    /// If the variable is not set or invalid, no proxy is configured.
    ///
    /// # Example
    /// ```
    /// use ntrip_core::NtripConfig;
    ///
    /// // Will use proxy from $HTTP_PROXY if set
    /// let config = NtripConfig::new("caster.example.com", 2101, "MOUNT")
    ///     .with_proxy_from_env();
    /// ```
    pub fn with_proxy_from_env(mut self) -> Self {
        self.proxy = ProxyConfig::from_env();
        self
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), Error> {
        if self.host.is_empty() {
            return Err(Error::InvalidConfig {
                message: "Host cannot be empty".to_string(),
            });
        }
        if self.port == 0 {
            return Err(Error::InvalidConfig {
                message: "Port cannot be 0".to_string(),
            });
        }
        // Validate against header injection (control characters)
        Self::validate_no_control_chars(&self.host, "host")?;
        Self::validate_no_control_chars(&self.mountpoint, "mountpoint")?;
        if let Some(ref u) = self.username {
            Self::validate_no_control_chars(u, "username")?;
        }
        if let Some(ref p) = self.password {
            Self::validate_no_control_chars(p, "password")?;
        }
        Ok(())
    }

    /// Validate that a string contains no ASCII control characters (header injection prevention).
    fn validate_no_control_chars(s: &str, field_name: &str) -> Result<(), Error> {
        if s.bytes().any(|b| b < 0x20 || b == 0x7F) {
            return Err(Error::InvalidConfig {
                message: format!(
                    "{} contains invalid control characters (possible header injection)",
                    field_name
                ),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_from_url_simple() {
        let proxy = ProxyConfig::from_url("proxy.example.com:8080").unwrap();
        assert_eq!(proxy.host, "proxy.example.com");
        assert_eq!(proxy.port, 8080);
        assert!(proxy.username.is_none());
        assert!(proxy.password.is_none());
    }

    #[test]
    fn test_proxy_config_from_url_with_http_prefix() {
        let proxy = ProxyConfig::from_url("http://proxy.example.com:3128").unwrap();
        assert_eq!(proxy.host, "proxy.example.com");
        assert_eq!(proxy.port, 3128);
    }

    #[test]
    fn test_proxy_config_from_url_with_credentials() {
        let proxy = ProxyConfig::from_url("http://user:pass@proxy.example.com:8080").unwrap();
        assert_eq!(proxy.host, "proxy.example.com");
        assert_eq!(proxy.port, 8080);
        assert_eq!(proxy.username.as_deref(), Some("user"));
        assert_eq!(proxy.password.as_deref(), Some("pass"));
    }

    #[test]
    fn test_proxy_config_from_url_default_port() {
        let proxy = ProxyConfig::from_url("proxy.example.com").unwrap();
        assert_eq!(proxy.host, "proxy.example.com");
        assert_eq!(proxy.port, 8080); // Default proxy port
    }

    #[test]
    fn test_proxy_config_from_url_empty_returns_none() {
        assert!(ProxyConfig::from_url("").is_none());
    }

    #[test]
    fn test_proxy_config_builder() {
        let proxy = ProxyConfig::new("proxy.local", 8888).with_credentials("admin", "secret");
        assert_eq!(proxy.host, "proxy.local");
        assert_eq!(proxy.port, 8888);
        assert_eq!(proxy.username.as_deref(), Some("admin"));
        assert_eq!(proxy.password.as_deref(), Some("secret"));
    }

    #[test]
    fn test_proxy_password_redacted_in_debug() {
        let proxy =
            ProxyConfig::new("proxy.local", 8080).with_credentials("user", "secret_password");
        let debug_output = format!("{:?}", proxy);
        assert!(!debug_output.contains("secret_password"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn test_ntrip_config_with_proxy() {
        let proxy = ProxyConfig::new("proxy.local", 8080);
        let config = NtripConfig::new("caster.example.com", 2101, "MOUNT").with_proxy(proxy);
        assert!(config.proxy.is_some());
        assert_eq!(config.proxy.as_ref().unwrap().host, "proxy.local");
    }
}
