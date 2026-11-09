//! The beta19 analytics protocol: CBOR request, SCM_RIGHTS descriptor, and CBOR snapshot.

use super::Snapshot;
use anyhow::{Context, ensure};
use nix::sys::socket::{ControlMessageOwned, MsgFlags, recvmsg};
use serde::Deserialize;
use std::io::IoSliceMut;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::FileExt;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, Interest};
use tokio::net::UnixStream;

const MAX_SNAPSHOT: usize = 2 * 1024 * 1024;

fn receive(stream: &UnixStream, header: &mut [u8; 4]) -> std::io::Result<(usize, Vec<OwnedFd>)> {
    let mut io = [IoSliceMut::new(header)];
    // Linux permits at most 253 SCM_RIGHTS descriptors in one message.
    let mut control = nix::cmsg_space!([i32; 253]);
    let message = recvmsg::<()>(
        stream.as_raw_fd(),
        &mut io,
        Some(&mut control),
        MsgFlags::MSG_DONTWAIT | MsgFlags::MSG_CMSG_CLOEXEC,
    )?;
    let mut fds = Vec::new();
    for cmsg in message.cmsgs()? {
        if let ControlMessageOwned::ScmRights(raw) = cmsg {
            for fd in raw {
                // SAFETY: SCM_RIGHTS transfers ownership of each fresh descriptor.
                fds.push(unsafe { OwnedFd::from_raw_fd(fd) });
            }
        }
    }
    Ok((message.bytes, fds))
}

fn decode<T: serde::de::DeserializeOwned>(mut bytes: &[u8]) -> anyhow::Result<T> {
    let value = ciborium::from_reader(&mut bytes).context("invalid analytics CBOR")?;
    ensure!(bytes.is_empty(), "trailing analytics data");
    Ok(value)
}

pub async fn read_snapshot(path: &Path) -> anyhow::Result<Snapshot> {
    tokio::time::timeout(Duration::from_secs(2), async {
        let mut stream = UnixStream::connect(path)
            .await
            .context("analytics socket unavailable")?;
        // Snapshot is read-only. A limit of 1 bounds unrelated host and flow rows.
        let mut request = Vec::new();
        ciborium::into_writer(
            &serde_json::json!({"kind":"snapshot","limit":1}),
            &mut request,
        )?;
        stream
            .write_all(&(request.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(&request).await?;
        let mut header = [0; 4];
        let (received, mut fds) = loop {
            stream.readable().await?;
            match stream.try_io(Interest::READABLE, || receive(&stream, &mut header)) {
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                result => break result?,
            }
        };
        ensure!(
            received > 0 && fds.len() == 1,
            "analytics reply needs one descriptor"
        );
        let file = std::fs::File::from(fds.pop().unwrap());
        stream.read_exact(&mut header[received..]).await?;
        let length = u32::from_be_bytes(header) as usize;
        ensure!(
            (1..=1024).contains(&length),
            "analytics reply header too large"
        );
        let mut reply = vec![0; length];
        stream.read_exact(&mut reply).await?;
        #[derive(Deserialize)]
        struct Reply {
            kind: Option<String>,
            snapshot_len: usize,
        }
        let reply: Reply = decode(&reply)?;
        ensure!(
            reply.kind.as_deref().is_none_or(|kind| kind == "snapshot"),
            "unexpected analytics reply kind"
        );
        ensure!(
            (1..=MAX_SNAPSHOT).contains(&reply.snapshot_len),
            "analytics snapshot too large"
        );
        let meta = file.metadata()?;
        ensure!(
            meta.is_file() && meta.len() >= reply.snapshot_len as u64,
            "invalid analytics descriptor length"
        );
        // Read at offset zero without mmap: a truncated producer file returns an error.
        let mut bytes = vec![0; reply.snapshot_len];
        file.read_exact_at(&mut bytes, 0)?;
        let snapshot: Snapshot = decode(&bytes)?;
        snapshot.validate()?;
        Ok(snapshot)
    })
    .await
    .context("analytics request timed out")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use nix::sys::socket::{ControlMessage, sendmsg};
    use serde_json::{Value, json};
    use std::io::{IoSlice, Read, Write};
    use std::os::unix::net::UnixListener;

    fn serve(
        value: Value,
        kind: &str,
        length: Option<usize>,
        count: usize,
    ) -> (tempfile::TempDir, std::thread::JoinHandle<()>) {
        let dir = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(dir.path().join("analytics.sock")).unwrap();
        let kind = kind.to_owned();
        let file_path = dir.path().join("snapshot.cbor");
        let thread = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(4)))
                .unwrap();
            let mut head = [0; 4];
            socket.read_exact(&mut head).unwrap();
            let mut request = vec![0; u32::from_be_bytes(head) as usize];
            socket.read_exact(&mut request).unwrap();
            let request: Value = decode(&request).unwrap();
            assert_eq!(request["kind"], "snapshot");
            assert_eq!(request["limit"], 1);
            let mut bytes = Vec::new();
            ciborium::into_writer(&value, &mut bytes).unwrap();
            let mut file = std::fs::File::options()
                .create(true)
                .truncate(true)
                .read(true)
                .write(true)
                .open(file_path)
                .unwrap();
            file.write_all(&bytes).unwrap();
            let mut reply = Vec::new();
            ciborium::into_writer(
                &json!({"kind":kind,"snapshot_len":length.unwrap_or(bytes.len())}),
                &mut reply,
            )
            .unwrap();
            let header = (reply.len() as u32).to_be_bytes();
            let fds = vec![file.as_raw_fd(); count];
            let rights = [ControlMessage::ScmRights(&fds)];
            // A fragmented header catches clients that assume recvmsg always returns four bytes.
            sendmsg::<()>(
                socket.as_raw_fd(),
                &[IoSlice::new(&header[..1])],
                if count == 0 { &[] } else { &rights },
                MsgFlags::empty(),
                None,
            )
            .unwrap();
            let _ = socket.write_all(&header[1..]);
            let _ = socket.write_all(&reply);
        });
        (dir, thread)
    }

    fn sample() -> Value {
        json!({"captured_at_millis":100,"workers":[{"queue_id":2,"busy_ppm":850000,"idle_ppm":150000,"backlog_mean":12}],"hosts":[{"private":"ignored"}]})
    }

    #[tokio::test]
    async fn worker_util_ipc_reads_a_fragmented_reply_and_ignores_unrelated_data() {
        let (dir, server) = serve(sample(), "snapshot", None, 1);
        let sample = read_snapshot(&dir.path().join("analytics.sock"))
            .await
            .unwrap();
        server.join().unwrap();
        assert_eq!(sample.workers[0].busy_ppm, 850000);
        assert_eq!(sample.workers[0].backlog_mean, Some(12));
    }

    #[tokio::test]
    async fn worker_util_ipc_rejects_invalid_sizes_kinds_and_descriptors() {
        for (kind, length, count) in [
            ("health", None, 1),
            ("snapshot", Some(MAX_SNAPSHOT + 1), 1),
            ("snapshot", Some(0), 1),
            ("snapshot", Some(1), 1),
            ("snapshot", None, 0),
            ("snapshot", None, 2),
        ] {
            let (dir, server) = serve(sample(), kind, length, count);
            assert!(
                read_snapshot(&dir.path().join("analytics.sock"))
                    .await
                    .is_err()
            );
            server.join().unwrap();
            // Both the server and client must release every received descriptor on failure.
            let file = dir.path().join("snapshot.cbor");
            let open = std::fs::read_dir("/proc/self/fd")
                .unwrap()
                .filter_map(Result::ok)
                .filter(|fd| std::fs::read_link(fd.path()).is_ok_and(|path| path == file))
                .count();
            assert_eq!(open, 0, "descriptor leak for {kind}/{length:?}/{count}");
        }
    }

    #[tokio::test]
    async fn worker_util_ipc_old_bridge_is_unavailable_instead_of_zero() {
        let (dir, server) = serve(json!({"captured_at_millis":100}), "snapshot", None, 1);
        assert!(
            read_snapshot(&dir.path().join("analytics.sock"))
                .await
                .unwrap_err()
                .to_string()
                .contains("no worker")
        );
        server.join().unwrap();
    }

    #[tokio::test]
    async fn worker_util_ipc_times_out_when_the_peer_never_answers() {
        let dir = tempfile::tempdir().unwrap();
        let listener = UnixListener::bind(dir.path().join("socket")).unwrap();
        let server = std::thread::spawn(move || {
            let (_socket, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(2300));
        });
        let error = read_snapshot(&dir.path().join("socket")).await.unwrap_err();
        assert!(error.to_string().contains("timed out"), "{error:#}");
        server.join().unwrap();
    }
}
