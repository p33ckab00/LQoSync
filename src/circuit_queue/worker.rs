//! One cancellable read per TUI. Socket permissions belong to the current user.
use super::{Done, Request};
use std::{path::PathBuf, sync::mpsc, thread::JoinHandle, time::Duration};

pub struct Worker {
    request: Request,
    rx: mpsc::Receiver<Done>,
    cancel: Option<tokio::sync::oneshot::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn start(config: PathBuf, request: Request) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let (cancel, cancelled) = tokio::sync::oneshot::channel();
        let job = request.clone();
        let thread = std::thread::Builder::new().name("tui-circuit-queue".into()).spawn(move || {
            let result = (|| -> anyhow::Result<super::Observation> {
                let (cfg, _) = crate::cli::read_config(&config)?;
                anyhow::ensure!(crate::detect::resolve(&cfg.libreqos)?.target != crate::config::Target::V2Pair, "Queue diagnostics require LibreQoS v3");
                let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
                runtime.block_on(async {
                    tokio::select! {
                        _ = cancelled => anyhow::bail!("Queue read cancelled"),
                        result = tokio::time::timeout(Duration::from_secs(7), super::fetch(std::path::Path::new(&cfg.libreqos.v3_analytics_socket), &job.circuit)) => result.map_err(|_| anyhow::anyhow!("Queue read timed out"))?,
                    }
                })
            })().map_err(|e| format!("{e:#}"));
            let _ = tx.send(Done { request: job, result });
        })?;
        Ok(Self {
            request,
            rx,
            cancel: Some(cancel),
            thread: Some(thread),
        })
    }
    pub fn poll(&self) -> Option<Done> {
        match self.rx.try_recv() {
            Ok(done) => Some(done),
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(Done {
                request: self.request.clone(),
                result: Err("Queue worker stopped without a result".into()),
            }),
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        if let Some(cancel) = self.cancel.take() {
            let _ = cancel.send(());
        }
        if let Some(thread) = self.thread.take().filter(|t| t.is_finished()) {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn circuit_queue_worker_drop_cancels_an_inflight_socket_without_waiting() {
        use std::io::Read;
        let dir = tempfile::tempdir().unwrap();
        let socket = dir.path().join("analytics.sock");
        let listener = std::os::unix::net::UnixListener::bind(&socket).unwrap();
        let config = dir.path().join("config.toml");
        std::fs::write(&config,format!("[libreqos]\ntarget=\"v3_pair\"\nv3_analytics_socket=\"{}\"\n[[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n", socket.display())).unwrap();
        let mut worker = Worker::start(
            config,
            Request {
                id: 1,
                circuit: "c1".into(),
            },
        )
        .unwrap();
        listener.set_nonblocking(true).unwrap();
        let end = std::time::Instant::now() + Duration::from_secs(2);
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(std::time::Instant::now() < end);
                    std::thread::sleep(Duration::from_millis(5));
                }
                Err(e) => panic!("{e}"),
            }
        };
        let thread = worker.thread.take().unwrap();
        let before = std::time::Instant::now();
        drop(worker);
        assert!(before.elapsed() < Duration::from_millis(100));
        stream
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        thread.join().unwrap();
    }
    #[test]
    fn circuit_queue_worker_returns_config_errors_without_blocking_the_caller() {
        let worker = Worker::start(
            PathBuf::from("/nonexistent/lqosync-queue-config"),
            Request {
                id: 1,
                circuit: "c1".into(),
            },
        )
        .unwrap();
        let end = std::time::Instant::now() + Duration::from_secs(2);
        let done = loop {
            if let Some(done) = worker.poll() {
                break done;
            }
            assert!(std::time::Instant::now() < end);
            std::thread::sleep(Duration::from_millis(5));
        };
        assert!(done.result.is_err());
        assert_eq!(done.request.id, 1);
    }
}
