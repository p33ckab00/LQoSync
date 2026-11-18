//! A non-blocking link from the TUI to the service API. Requests run on their own thread.

use serde_json::Value;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

/// One answer, in request order. The error text is ready for display.
#[derive(Debug)]
pub struct ApiAnswer {
    pub op: String,
    pub result: Result<Value, String>,
}

pub struct ApiLink {
    requests: Sender<(String, Value)>,
    answers: Receiver<ApiAnswer>,
}

impl ApiLink {
    /// Start the request thread. It runs one request at a time, in order.
    pub fn spawn(state_dir: PathBuf) -> Self {
        let (requests, queue) = channel::<(String, Value)>();
        let (reply, answers) = channel();
        std::thread::spawn(move || {
            let Ok(runtime) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            for (op, args) in queue {
                let result = runtime
                    .block_on(crate::api::client::request(
                        &state_dir,
                        &op,
                        args,
                        crate::api::client::DEFAULT_TIMEOUT,
                    ))
                    .map_err(|e| e.to_string());
                if reply.send(ApiAnswer { op, result }).is_err() {
                    return;
                }
            }
        });
        Self { requests, answers }
    }

    /// Queue a request. This call never waits for the service.
    pub fn send(&self, op: &str, args: Value) {
        let _ = self.requests.send((op.to_string(), args));
    }

    /// The next answer, when one is ready.
    pub fn try_recv(&self) -> Option<ApiAnswer> {
        self.answers.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::{Duration, Instant};

    async fn wait_for(link: &ApiLink) -> ApiAnswer {
        let start = Instant::now();
        loop {
            if let Some(a) = link.try_recv() {
                return a;
            }
            assert!(start.elapsed() < Duration::from_secs(10), "no answer");
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn answers_come_back_in_order_without_blocking_the_caller() {
        let dir = tempfile::tempdir().unwrap();
        let held = crate::files::InstanceLock::acquire(dir.path()).unwrap();
        let (listener, _g) = crate::api::server::bind(dir.path(), &held).unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<crate::api::server::Job>(16);
        crate::api::server::spawn(listener, tx);
        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                tokio::time::sleep(Duration::from_millis(50)).await;
                let line =
                    crate::api::protocol::response_ok(&job.request.id, json!(job.request.op));
                let _ = job.reply.send(line);
            }
        });
        let link = ApiLink::spawn(dir.path().to_path_buf());
        let sent = Instant::now();
        link.send("first", json!({}));
        link.send("second", json!({}));
        assert!(
            sent.elapsed() < Duration::from_millis(40),
            "send must not wait"
        );
        let a = wait_for(&link).await;
        let b = wait_for(&link).await;
        assert_eq!(
            (a.op.as_str(), a.result.unwrap()),
            ("first", json!("first"))
        );
        assert_eq!(
            (b.op.as_str(), b.result.unwrap()),
            ("second", json!("second"))
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_missing_socket_gives_a_not_running_answer() {
        let dir = tempfile::tempdir().unwrap();
        let link = ApiLink::spawn(dir.path().to_path_buf());
        link.send("status", json!({}));
        let a = wait_for(&link).await;
        assert!(a.result.unwrap_err().contains("does not answer"));
    }
}
