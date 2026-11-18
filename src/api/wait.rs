//! Serve API jobs while the service waits for its next cycle deadline.

use super::dispatch::LostLock;
use super::protocol::Request;
use super::server::Job;
use tokio::sync::mpsc::Receiver;
use tokio::time::{Instant, sleep_until};

/// Answer jobs until `deadline`. The deadline does not move when jobs arrive.
/// A lost lock answers the job and returns at once.
pub async fn serve_until<F>(
    deadline: Instant,
    jobs: &mut Receiver<Job>,
    mut handle: F,
) -> Result<(), LostLock>
where
    F: FnMut(&Request) -> Result<String, LostLock>,
{
    loop {
        tokio::select! {
            () = sleep_until(deadline) => return Ok(()),
            job = jobs.recv() => {
                let Some(job) = job else {
                    // No more connections can reach the service. Keep the cycle cadence.
                    sleep_until(deadline).await;
                    return Ok(());
                };
                match handle(&job.request) {
                    Ok(line) => {
                        let _ = job.reply.send(line);
                    }
                    Err(LostLock(line)) => {
                        let _ = job.reply.send(line.clone());
                        return Err(LostLock(line));
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::protocol::{Request, response_ok};
    use crate::api::server::Job;
    use serde_json::json;
    use tokio::sync::{mpsc, oneshot};
    use tokio::time::{Duration, Instant};

    fn job(op: &str) -> (Job, oneshot::Receiver<String>) {
        let (reply, rx) = oneshot::channel();
        let request = Request {
            v: 1,
            id: op.into(),
            op: op.into(),
            args: json!({}),
        };
        (Job { request, reply }, rx)
    }

    #[tokio::test(start_paused = true)]
    async fn jobs_are_answered_and_the_deadline_does_not_move() {
        let (tx, mut rx) = mpsc::channel(16);
        let start = Instant::now();
        let deadline = start + Duration::from_secs(20);
        let client = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let (j, answer) = job("status");
            tx.send(j).await.unwrap();
            let line = answer.await.unwrap();
            (Instant::now() - start, line)
        });
        serve_until(deadline, &mut rx, |r| Ok(response_ok(&r.id, json!("done"))))
            .await
            .unwrap();
        assert_eq!(Instant::now() - start, Duration::from_secs(20));
        let (answered_at, line) = client.await.unwrap();
        assert_eq!(answered_at, Duration::from_secs(5));
        assert!(line.contains("done"));
    }

    #[tokio::test(start_paused = true)]
    async fn a_job_queued_during_a_cycle_is_answered_after_it() {
        let (tx, mut rx) = mpsc::channel(16);
        let (j, answer) = job("status");
        tx.send(j).await.unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        serve_until(deadline, &mut rx, |r| Ok(response_ok(&r.id, json!(1))))
            .await
            .unwrap();
        assert!(answer.await.unwrap().contains("\"ok\":true"));
    }

    #[tokio::test(start_paused = true)]
    async fn a_lost_lock_stops_serving() {
        let (tx, mut rx) = mpsc::channel(16);
        let (j, answer) = job("status");
        tx.send(j).await.unwrap();
        let start = Instant::now();
        let deadline = start + Duration::from_secs(20);
        let r = serve_until(deadline, &mut rx, |_| Err(LostLock("lost\n".into()))).await;
        assert!(r.is_err());
        assert!(Instant::now() - start < Duration::from_secs(20));
        assert_eq!(answer.await.unwrap(), "lost\n");
    }
}
