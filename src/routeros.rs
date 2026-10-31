//! RouterOS reads: the error type, the session traits, and the snapshot read.

use std::fmt;

/// Why a router read failed. The text never holds a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReadError {
    /// TCP connect refused or timed out: the router is not reachable.
    Unreachable(String),
    /// The router refused the login.
    Login(String),
    /// TLS handshake or certificate problem.
    Tls(String),
    /// A command did not complete in `api_timeout`.
    Timeout(String),
    /// The router answered `!trap`.
    Trap(String),
    /// Any other protocol or connection problem.
    Protocol(String),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, text) = match self {
            ReadError::Unreachable(t) => ("unreachable", t),
            ReadError::Login(t) => ("login refused", t),
            ReadError::Tls(t) => ("TLS error", t),
            ReadError::Timeout(t) => ("timeout", t),
            ReadError::Trap(t) => ("router error", t),
            ReadError::Protocol(t) => ("protocol error", t),
        };
        write!(f, "{kind}: {text}")
    }
}
