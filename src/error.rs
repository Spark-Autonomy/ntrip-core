//! Error types for ntrip-core.

use thiserror::Error;

/// Errors that can occur during NTRIP operations.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Failed to connect to the NTRIP caster.
    #[error("Failed to connect to {host}:{port}: {source}")]
    ConnectionFailed {
        host: String,
        port: u16,
        #[source]
        source: std::io::Error,
    },

    /// Connection timed out.
    #[error("Connection timed out after {timeout_secs} seconds")]
    Timeout { timeout_secs: u32 },

    /// Read timed out - no data received within configured period.
    #[error("Read timed out after {timeout_secs} seconds - no data received")]
    ReadTimeout { timeout_secs: u32 },

    /// Authentication failed.
    #[error("Authentication failed for {host} (user: {username})")]
    AuthenticationFailed { host: String, username: String },

    /// Mountpoint not found on the caster.
    #[error("Mountpoint '{mountpoint}' not found on {host}")]
    MountpointNotFound { host: String, mountpoint: String },

    /// HTTP error response from server.
    #[error("HTTP error from {host}: {status_code} {reason}")]
    HttpError {
        host: String,
        status_code: u16,
        reason: String,
    },

    /// TLS/HTTPS error.
    #[error("TLS error: {message}")]
    TlsError { message: String },

    /// Network I/O error.
    #[error("Network error: {source}")]
    NetworkError {
        #[source]
        source: std::io::Error,
    },

    /// Stream disconnected.
    #[error("Stream disconnected: {reason}")]
    StreamDisconnected { reason: String },

    /// Invalid configuration.
    #[error("Invalid configuration: {message}")]
    InvalidConfig { message: String },

    /// Failed to parse sourcetable.
    #[error("Failed to parse sourcetable: {message}")]
    SourcetableParseError { message: String },
}

impl Error {
    /// Create a connection failed error.
    pub fn connection_failed(host: &str, port: u16, source: std::io::Error) -> Self {
        Self::ConnectionFailed {
            host: host.to_string(),
            port,
            source,
        }
    }
}
