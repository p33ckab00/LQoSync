//! Serve API jobs while the service waits for its next cycle deadline.

use super::dispatch::LostLock;
use super::protocol::Request;
use super::server::Job;
use tokio::sync::mpsc::Receiver;
use tokio::time::{Instant, sleep_until};

/// Answers one API request on the task that owns the service state.
pub trait Handler {
    fn handle(
        &mut self,
        request: &Request,
    ) -> impl std::future::Future<Output = Result<String, LostLock>>;
}

impl<C> Handler for crate::runner::Runner<C>
where
    C: crate::routeros::Connector + 'static,
    C::Session: 'static,
{
    async fn handle(&mut self, request: &Request) -> Result<String, LostLock> {
        super::dispatch::handle(self, request).await
    }
}

/// Answer jobs until `deadline`. The deadline does not move when jobs arrive.
/// A job that runs past the deadline returns at its end, without a second wait.
/// A lost lock answers the job and returns at once.
pub async fn serve_until(
    deadline: Instant,
    jobs: &mut Receiver<Job>,
    handler: &mut impl Handler,
) -> Result<(), LostLock> {
    loop {
        tokio::select! {
            () = sleep_until(deadline) => return Ok(()),
            job = jobs.recv() => {
                let Some(job) = job else {
                    // No more connections can reach the service. Keep the cycle cadence.
                    sleep_until(deadline).await;
                    return Ok(());
                };
                match handler.handle(&job.request).await {
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

    /// A test handler: answer after `delay`, or report a lost lock.
    struct Fake {
        delay: Duration,
        lost: bool,
    }
    impl Handler for Fake {
        async fn handle(&mut self, r: &Request) -> Result<String, LostLock> {
            tokio::time::sleep(self.delay).await;
            if self.lost {
                return Err(LostLock("lost\n".into()));
            }
            Ok(response_ok(&r.id, json!("done")))
        }
    }
    fn quick() -> Fake {
        Fake {
            delay: Duration::ZERO,
            lost: false,
        }
    }

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
        serve_until(deadline, &mut rx, &mut quick()).await.unwrap();
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
        serve_until(deadline, &mut rx, &mut quick()).await.unwrap();
        assert!(answer.await.unwrap().contains("\"ok\":true"));
    }

    #[tokio::test(start_paused = true)]
    async fn a_lost_lock_stops_serving() {
        let (tx, mut rx) = mpsc::channel(16);
        let (j, answer) = job("status");
        tx.send(j).await.unwrap();
        let start = Instant::now();
        let deadline = start + Duration::from_secs(20);
        let mut lost = Fake {
            delay: Duration::ZERO,
            lost: true,
        };
        let r = serve_until(deadline, &mut rx, &mut lost).await;
        assert!(r.is_err());
        assert!(Instant::now() - start < Duration::from_secs(20));
        assert_eq!(answer.await.unwrap(), "lost\n");
    }

    #[tokio::test(start_paused = true)]
    async fn an_async_job_that_outlasts_the_deadline_returns_after_it_without_a_second_wait() {
        let (tx, mut rx) = mpsc::channel(16);
        let start = Instant::now();
        let deadline = start + Duration::from_secs(10);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let (j, answer) = job("topology.preview.refresh");
            tx.send(j).await.unwrap();
            answer.await.unwrap();
        });
        let mut slow = Fake {
            delay: Duration::from_secs(30),
            lost: false,
        };
        serve_until(deadline, &mut rx, &mut slow).await.unwrap();
        assert_eq!(Instant::now() - start, Duration::from_secs(35));
    }
}
