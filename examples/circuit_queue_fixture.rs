//! Isolated source for actual-terminal captures. All records are synthetic.
use nix::sys::socket::{ControlMessage, MsgFlags, sendmsg};
use serde_json::json;
use std::{
    io::{IoSlice, Read, Write},
    os::{fd::AsRawFd, unix::net::UnixListener},
};
fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("synthetic socket path");
    let listener = UnixListener::bind(path)?;
    let mut slots = Vec::new();
    ciborium::into_writer(
        &(0..32).map(|i| format!("SUB-{i:04}")).collect::<Vec<_>>(),
        &mut slots,
    )?;
    for socket in listener.incoming() {
        let mut socket = socket?;
        socket.set_read_timeout(Some(std::time::Duration::from_secs(2)))?;
        let result = (|| -> anyhow::Result<()> {
            let mut head = [0; 4];
            socket.read_exact(&mut head)?;
            let size = u32::from_be_bytes(head) as usize;
            anyhow::ensure!(size <= 4096, "fixture request too large");
            let mut request = vec![0; size];
            socket.read_exact(&mut request)?;
            let request: serde_json::Value = ciborium::from_reader(request.as_slice())?;
            let kind = request["kind"].as_str().unwrap_or("");
            let now = lqosync3::circuit_queue::now_millis();
            let body = match kind {
                "state" => {
                    json!({"Report":{"generation":"synthetic-capture","report_schema":1,"slot_order":slots}})
                }
                "queue_stats" => {
                    anyhow::ensure!(
                        request["limit"] == 1
                            && request["queue_slots"]
                                .as_array()
                                .is_some_and(|v| v.len() == 1),
                        "one synthetic slot required"
                    );
                    let slot = request["queue_slots"][0].as_u64().unwrap_or(0);
                    anyhow::ensure!((1..=32).contains(&slot), "synthetic slot out of range");
                    let dir = json!({"queue_depth_pkts":12,"drop_pkts":42,"drop_pkts_queue_full":3,"aqm_marks":7,"memory_used":958104,"tins":(0..8).map(|i|json!({"sent_bytes":120000+i*1000,"backlog_bytes":if i==0{4096}else{0},"sojourn_p99_ns":1500000+i*100000})).collect::<Vec<_>>()});
                    json!({"rows":[{"slot_id":slot,"download":dir,"upload":dir,"rtt_histogram":(0..50).map(|i|if i==1{8}else{0}).collect::<Vec<_>>()}],"sampled_at_unix_ms":now,"elapsed_ms":200,"cadence_ms":200,"sample_epoch_unix_ms":now-10000,"sample_seq":50,"actor_gap":false,"counter_reset":false})
                }
                _ => anyhow::bail!("synthetic endpoint unavailable"),
            };
            let mut bytes = Vec::new();
            ciborium::into_writer(&body, &mut bytes)?;
            let mut file = tempfile::tempfile()?;
            file.write_all(&bytes)?;
            let mut reply = Vec::new();
            ciborium::into_writer(&json!({"kind":kind,"snapshot_len":bytes.len()}), &mut reply)?;
            let mut framed = (reply.len() as u32).to_be_bytes().to_vec();
            framed.extend(reply);
            let fds = [file.as_raw_fd()];
            sendmsg::<()>(
                socket.as_raw_fd(),
                &[IoSlice::new(&framed[..1])],
                &[ControlMessage::ScmRights(&fds)],
                MsgFlags::empty(),
                None,
            )?;
            socket.write_all(&framed[1..])?;
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("synthetic fixture: {e}");
        }
    }
    Ok(())
}
