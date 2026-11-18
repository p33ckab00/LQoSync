//! Client for the service API. The CLI uses it now; the TUI uses it in Phase 2b.

use super::protocol::{ApiError, MAX_RESPONSE, VERSION, read_line};
use super::server::SOCKET_NAME;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Default time to wait for an answer. A request waits while a cycle runs.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Debug)]
pub enum ClientError {
    NotRunning(PathBuf),
    Timeout,
    Api(ApiError),
    Protocol(String),
}
impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotRunning(path) => {
                write!(f, "lqosync3.service does not answer on {}", path.display())
            }
            Self::Timeout => f.write_str("the service did not answer in time"),
            Self::Api(e) => write!(f, "{e}"),
            Self::Protocol(m) => write!(f, "invalid API answer: {m}"),
        }
    }
}
impl std::error::Error for ClientError {}

/// Send one request and return its result.
pub async fn request(
    state_dir: &Path,
    op: &str,
    args: Value,
    timeout: Duration,
) -> Result<Value, ClientError> {
    let path = state_dir.join(SOCKET_NAME);
    let exchange = async {
        let stream = UnixStream::connect(&path)
            .await
            .map_err(|_| ClientError::NotRunning(path.clone()))?;
        let (read, mut write) = stream.into_split();
        let id = format!("client-{}", std::process::id());
        let mut line =
            serde_json::to_string(&json!({"v": VERSION, "id": id, "op": op, "args": args}))
                .map_err(|e| ClientError::Protocol(e.to_string()))?;
        line.push('\n');
        write
            .write_all(line.as_bytes())
            .await
            .map_err(|_| ClientError::NotRunning(path.clone()))?;
        let answer = read_line(&mut BufReader::new(read), MAX_RESPONSE)
            .await
            .map_err(|e| ClientError::Protocol(e.to_string()))?
            .ok_or_else(|| {
                ClientError::Protocol("the connection closed without an answer".into())
            })?;
        let value: Value =
            serde_json::from_slice(&answer).map_err(|e| ClientError::Protocol(e.to_string()))?;
        if value["ok"] == Value::Bool(true) {
            Ok(value["result"].clone())
        } else {
            let error: ApiError = serde_json::from_value(value["error"].clone())
                .map_err(|e| ClientError::Protocol(e.to_string()))?;
            Err(ClientError::Api(error))
        }
    };
    tokio::time::timeout(timeout, exchange)
        .await
        .map_err(|_| ClientError::Timeout)?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::protocol::{ApiError, ErrorCode, response_err};
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn a_missing_socket_is_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let e = request(dir.path(), "status", json!({}), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert!(matches!(e, ClientError::NotRunning(_)), "{e}");
        assert!(e.to_string().contains("lqosync3.service does not answer"));
    }

    #[tokio::test]
    async fn an_api_error_keeps_its_code() {
        let dir = tempfile::tempdir().unwrap();
        let held = crate::files::InstanceLock::acquire(dir.path()).unwrap();
        let (listener, _g) = crate::api::server::bind(dir.path(), &held).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::api::server::Job>(4);
        crate::api::server::spawn(listener, tx);
        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                let e = ApiError::new(ErrorCode::Conflict, "stale");
                let _ = job.reply.send(response_err(&job.request.id, &e));
            }
        });
        let e = request(
            dir.path(),
            "policy.commit",
            json!({}),
            Duration::from_secs(5),
        )
        .await
        .unwrap_err();
        assert!(
            matches!(&e, ClientError::Api(a) if a.code == ErrorCode::Conflict),
            "{e}"
        );
    }
}
