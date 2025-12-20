//! Configuration types for ntrip-core.

use crate::Error;
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
