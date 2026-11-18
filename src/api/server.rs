//! The socket server. Each accepted peer must be root or the service user.

use super::protocol::{
    ApiError, ErrorCode, MAX_REQUEST, Request, parse_line, read_line, response_err,
};
use anyhow::{Context, bail};
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, oneshot};

pub const SOCKET_NAME: &str = "api.sock";
/// A connection that sends no complete request in this time closes.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(60);

/// One request for the task that owns the `Runner`. The reply is one complete response line.
pub struct Job {
    pub request: Request,
    pub reply: oneshot::Sender<String>,
}

pub fn peer_allowed(peer_uid: u32, service_euid: u32) -> bool {
    peer_uid == 0 || peer_uid == service_euid
}

/// Removes the socket file when the service stops by any path.
pub struct SocketGuard {
    path: PathBuf,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Bind `<state_dir>/api.sock` with mode 0600. The held instance lock proves ownership,
/// so a stale socket file can be replaced. A non-socket file at that path stops startup.
pub(crate) fn bind(
    state_dir: &Path,
    _lock_held: &crate::files::InstanceLock,
) -> anyhow::Result<(UnixListener, SocketGuard)> {
    let path = state_dir.join(SOCKET_NAME);
    match std::fs::symlink_metadata(&path) {
        Ok(m) if m.file_type().is_socket() => std::fs::remove_file(&path)
            .with_context(|| format!("cannot remove stale {}", path.display()))?,
        Ok(_) => bail!("{} exists and is not a socket", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).with_context(|| format!("cannot inspect {}", path.display())),
    }
    // Bind under a private name, restrict the mode, then publish with a rename.
    let staged = state_dir.join(format!(".{SOCKET_NAME}.{}", std::process::id()));
    let _ = std::fs::remove_file(&staged);
    let listener =
        UnixListener::bind(&staged).with_context(|| format!("cannot bind {}", staged.display()))?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&staged, &path)
        .with_context(|| format!("cannot publish {}", path.display()))?;
    Ok((listener, SocketGuard { path }))
}

/// Accept connections and forward each request to `jobs`. A full channel answers `busy`.
pub fn spawn(listener: UnixListener, jobs: mpsc::Sender<Job>) -> tokio::task::JoinHandle<()> {
    let euid = std::fs::metadata("/proc/self").map_or(0, |m| m.uid());
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(connection(stream, jobs.clone(), euid));
        }
    })
}

async fn connection(stream: UnixStream, jobs: mpsc::Sender<Job>, euid: u32) {
    let allowed = stream
        .peer_cred()
        .is_ok_and(|cred| peer_allowed(cred.uid(), euid));
    let (read, mut write) = stream.into_split();
    if !allowed {
        let error = ApiError::new(ErrorCode::Unauthorized, "only root can use this socket");
        let _ = write.write_all(response_err("", &error).as_bytes()).await;
        return;
    }
    let mut reader = BufReader::new(read);
    loop {
        let line =
            match tokio::time::timeout(IDLE_TIMEOUT, read_line(&mut reader, MAX_REQUEST)).await {
                Err(_) | Ok(Ok(None)) => return,
                Ok(Err(e)) => {
                    let error = ApiError::new(ErrorCode::BadRequest, e.to_string());
                    let _ = write.write_all(response_err("", &error).as_bytes()).await;
                    return;
                }
                Ok(Ok(Some(line))) => line,
            };
        let answer = match parse_line(&line) {
            Err((id, error)) => response_err(&id, &error),
            Ok(request) => {
                let id = request.id.clone();
                let (reply, wait) = oneshot::channel();
                match jobs.try_send(Job { request, reply }) {
                    Err(mpsc::error::TrySendError::Full(_)) => response_err(
                        &id,
                        &ApiError::new(
                            ErrorCode::Busy,
                            "the service has too many waiting requests",
                        ),
                    ),
                    Err(mpsc::error::TrySendError::Closed(_)) => response_err(
                        &id,
                        &ApiError::new(ErrorCode::Internal, "the service is stopping"),
                    ),
                    Ok(()) => wait.await.unwrap_or_else(|_| {
                        response_err(&id, &ApiError::new(ErrorCode::Internal, "no reply"))
                    }),
                }
            }
        };
        if write.write_all(answer.as_bytes()).await.is_err() {
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    fn lock(dir: &Path) -> std::sync::Arc<crate::files::InstanceLock> {
        crate::files::InstanceLock::acquire(dir).unwrap()
    }

    async fn ask(path: &Path, line: &str) -> serde_json::Value {
        let mut s = UnixStream::connect(path).await.unwrap();
        s.write_all(line.as_bytes()).await.unwrap();
        let mut out = String::new();
        BufReader::new(s).read_line(&mut out).await.unwrap();
        serde_json::from_str(&out).unwrap()
    }

    #[test]
    fn peer_rule_accepts_root_and_the_service_uid_only() {
        assert!(peer_allowed(0, 1000));
        assert!(peer_allowed(1000, 1000));
        assert!(!peer_allowed(1001, 1000));
        assert!(peer_allowed(0, 0));
        assert!(!peer_allowed(1000, 0));
    }

    #[tokio::test]
    async fn bind_replaces_a_stale_socket() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path());
        let stale = std::os::unix::net::UnixListener::bind(dir.path().join("api.sock")).unwrap();
        drop(stale);
        let (_l, _g) = bind(dir.path(), &held).unwrap();
        assert!(
            std::fs::metadata(dir.path().join("api.sock"))
                .unwrap()
                .file_type()
                .is_socket()
        );
    }

    #[tokio::test]
    async fn bind_refuses_a_regular_file_at_the_socket_path() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path());
        std::fs::write(dir.path().join("api.sock"), "x").unwrap();
        assert!(bind(dir.path(), &held).is_err());
        assert_eq!(
            std::fs::read_to_string(dir.path().join("api.sock")).unwrap(),
            "x"
        );
    }

    #[tokio::test]
    async fn the_socket_has_mode_0600_and_the_guard_removes_it_on_drop() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path());
        let (listener, guard) = bind(dir.path(), &held).unwrap();
        let path = dir.path().join("api.sock");
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
        drop(listener);
        drop(guard);
        assert!(!path.exists());
    }

    #[tokio::test]
    async fn a_request_reaches_the_job_channel_and_the_reply_returns() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path());
        let (listener, _g) = bind(dir.path(), &held).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel(16);
        spawn(listener, tx);
        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                let _ = job.reply.send(crate::api::protocol::response_ok(
                    &job.request.id,
                    serde_json::json!(job.request.op),
                ));
            }
        });
        let v = ask(
            &dir.path().join("api.sock"),
            "{\"v\":1,\"id\":\"q\",\"op\":\"status\"}\n",
        )
        .await;
        assert_eq!(
            (v["id"].as_str(), v["result"].as_str()),
            (Some("q"), Some("status"))
        );
        let bad = ask(
            &dir.path().join("api.sock"),
            "{\"v\":9,\"id\":\"r\",\"op\":\"status\"}\n",
        )
        .await;
        assert_eq!(bad["error"]["code"], "unsupported_version");
    }

    #[tokio::test]
    async fn a_full_channel_answers_busy() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path());
        let (listener, _g) = bind(dir.path(), &held).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(1);
        spawn(listener, tx);
        let path = dir.path().join("api.sock");
        let mut first = UnixStream::connect(&path).await.unwrap();
        first
            .write_all(b"{\"v\":1,\"id\":\"a\",\"op\":\"status\"}\n")
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let v = ask(&path, "{\"v\":1,\"id\":\"b\",\"op\":\"status\"}\n").await;
        assert_eq!(v["error"]["code"], "busy");
    }

    #[tokio::test(start_paused = true)]
    async fn an_idle_connection_closes_after_the_timeout() {
        let dir = tempfile::tempdir().unwrap();
        let held = lock(dir.path());
        let (listener, _g) = bind(dir.path(), &held).unwrap();
        let (tx, _rx) = tokio::sync::mpsc::channel(16);
        spawn(listener, tx);
        let s = UnixStream::connect(dir.path().join("api.sock"))
            .await
            .unwrap();
        let mut out = String::new();
        let n = BufReader::new(s).read_line(&mut out).await.unwrap();
        assert_eq!(n, 0, "the server closes an idle connection");
    }
}
