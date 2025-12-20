//! Abstraction over plain TCP and TLS streams for NTRIP connections.

use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_rustls::client::TlsStream;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_rustls::TlsConnector;
use tracing::{debug, info};

use crate::Error;

/// A stream that can be either plain TCP or TLS-encrypted.
pub enum NtripStream {
    /// Plain TCP connection (no encryption)
    Plain(TcpStream),
    /// TLS-encrypted connection
    Tls(Box<TlsStream<TcpStream>>),
}

impl NtripStream {
    /// Create a plain TCP stream.
    pub fn plain(stream: TcpStream) -> Self {
        Self::Plain(stream)
    }

    /// Upgrade a TCP stream to TLS.
    pub async fn connect_tls(
        stream: TcpStream,
        host: &str,
        skip_verify: bool,
    ) -> Result<Self, Error> {
        let config = if skip_verify {
            debug!("TLS: Using insecure configuration (certificate verification disabled)");
            create_insecure_tls_config()
        } else {
            debug!("TLS: Using secure configuration with system root certificates");
            create_tls_config()
        }?;

        let connector = TlsConnector::from(Arc::new(config));

        // Parse the server name for SNI
        let server_name = ServerName::try_from(host.to_string()).map_err(|_| Error::TlsError {
            message: format!("Invalid server name for TLS: {}", host),
        })?;

        info!(host = %host, "Performing TLS handshake");

        let tls_stream =
            connector
                .connect(server_name, stream)
                .await
                .map_err(|e| Error::TlsError {
                    message: format!("TLS handshake failed: {}", e),
                })?;

        info!("TLS connection established");
        Ok(Self::Tls(Box::new(tls_stream)))
    }
}

/// Create a TLS configuration with system root certificates.
fn create_tls_config() -> Result<ClientConfig, Error> {
    let mut root_store = RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

/// Create an insecure TLS configuration that skips certificate verification.
/// WARNING: Only use for testing with self-signed certificates.
fn create_insecure_tls_config() -> Result<ClientConfig, Error> {
    use tokio_rustls::rustls::client::danger::{
        HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier,
    };
    use tokio_rustls::rustls::pki_types::{CertificateDer, UnixTime};
    use tokio_rustls::rustls::{DigitallySignedStruct, SignatureScheme};

    #[derive(Debug)]
    struct NoVerifier;

    impl ServerCertVerifier for NoVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, tokio_rustls::rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, tokio_rustls::rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::ECDSA_NISTP521_SHA512,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
                SignatureScheme::ED25519,
            ]
        }
    }

    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth();

    Ok(config)
}

// Implement AsyncRead for NtripStream
impl AsyncRead for NtripStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match self.get_mut() {
            NtripStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            NtripStream::Tls(stream) => Pin::new(stream.as_mut()).poll_read(cx, buf),
        }
    }
}

// Implement AsyncWrite for NtripStream
impl AsyncWrite for NtripStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match self.get_mut() {
            NtripStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            NtripStream::Tls(stream) => Pin::new(stream.as_mut()).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            NtripStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            NtripStream::Tls(stream) => Pin::new(stream.as_mut()).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        match self.get_mut() {
            NtripStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            NtripStream::Tls(stream) => Pin::new(stream.as_mut()).poll_shutdown(cx),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_creation() {
        let config = create_tls_config();
        assert!(config.is_ok());
    }

    #[test]
    fn test_insecure_tls_config_creation() {
        let config = create_insecure_tls_config();
        assert!(config.is_ok());
    }
}
