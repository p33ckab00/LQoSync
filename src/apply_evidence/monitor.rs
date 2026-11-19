//! One background checker. No RouterOS collection; file parsing runs outside the async executor.
use super::{Prepared, Proof, prepare, verify_prepared};
use crate::{
    config::Config,
    system_health::{Domain, Hub, Observation, Phase, State},
    write_receipt::{Receipt, Tracker},
};
use std::{
    os::unix::fs::MetadataExt,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
#[derive(Clone)]
struct Settings {
    cfg: Config,
    digest: String,
}
type Stamp = (u64, u64, u64, i64, i64, i64, i64);
fn stamp(path: &Path) -> Option<Stamp> {
    let m = std::fs::metadata(path).ok()?;
    Some((
        m.dev(),
        m.ino(),
        m.len(),
        m.mtime(),
        m.mtime_nsec(),
        m.ctime(),
        m.ctime_nsec(),
    ))
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn timeout(tracker: &Mutex<Tracker>, receipt: &Receipt, hub: &Hub) {
    let at = now();
    if at.saturating_sub(receipt.started) < crate::runner::IMPORT_LIMIT_SECS {
        return;
    }
    if let Ok(mut t) = tracker.try_lock()
        && t.current()
            .is_some_and(|r| r.write_id == receipt.write_id && r.phase.pending())
        && let Err(error) = t.transition(Phase::VerifyTimeout, at, hub)
    {
        eprintln!("apply verification receipt: {error}");
    }
}
pub struct Monitor {
    settings: watch::Sender<Settings>,
    task: tokio::task::JoinHandle<()>,
}
impl Monitor {
    pub fn start(cfg: &Config, digest: &str, tracker: Arc<Mutex<Tracker>>, hub: Hub) -> Self {
        let (settings, mut changes) = watch::channel(Settings {
            cfg: cfg.clone(),
            digest: digest.into(),
        });
        let task = tokio::spawn(async move {
            let mut current = changes.borrow().clone();
            let mut cached: Option<(String, Prepared)> = None;
            let mut imported: Option<(Stamp, bool)> = None;
            let mut tick = tokio::time::interval(Duration::from_secs(2));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                tokio::select! {
                    changed=changes.changed()=>{
                        if changed.is_err(){break;}
                        current=changes.borrow_and_update().clone();cached=None;imported=None;
                    }
                    _=tick.tick()=>{
                        let receipt=tracker.try_lock().ok().and_then(|t|t.current().cloned());
                        let Some(mut receipt)=receipt.filter(|r|r.phase.pending()||r.phase==Phase::VerifyTimeout) else {cached=None;imported=None;continue};
                        if matches!(receipt.phase,Phase::Intent|Phase::Interrupted)
                            && let Ok(mut t)=tracker.try_lock()
                                && t.current().is_some_and(|r|r.write_id==receipt.write_id) {
                                    if let Err(error)=t.reconcile(now(),&hub) {eprintln!("write reconciliation: {error}");}
                                    if let Some(recovered)=t.current() {receipt=recovered.clone();}
                                }
                        if !matches!(receipt.phase,Phase::Pending|Phase::ImportConfirmed|Phase::VerifyTimeout){timeout(&tracker,&receipt,&hub);continue}
                        if cached.as_ref().is_none_or(|(id,_)|*id!=receipt.write_id) {
                            let pending=receipt.clone();
                            match tokio::task::spawn_blocking(move||prepare(&pending)).await {
                                Ok(Ok(value))=>{cached=Some((receipt.write_id.clone(),value));imported=None;}
                                _=>{timeout(&tracker,&receipt,&hub);continue}
                            }
                        }
                        let path=Path::new(&current.cfg.libreqos.state_directory).join("topology/topology_import.json");
                        let Some(current_stamp)=stamp(&path) else {timeout(&tracker,&receipt,&hub);continue};
                        if imported.as_ref().is_none_or(|(old,_)|*old!=current_stamp) {
                            let expected=cached.as_ref().expect("prepared write").1.clone();
                            let matched=tokio::task::spawn_blocking(move||expected.import_matches(&path)).await;
                            imported=Some((current_stamp,matches!(matched,Ok(Ok(true)))));
                        }
                        if !imported.as_ref().is_some_and(|(_,matched)|*matched) {timeout(&tracker,&receipt,&hub);continue}
                        if changes.borrow().digest!=current.digest {continue}
                        if receipt.phase==Phase::Pending
                            && let Ok(mut t)=tracker.try_lock()
                                && t.current().is_some_and(|r|r.write_id==receipt.write_id && r.phase==Phase::Pending) {
                                    if let Err(error)=t.transition(Phase::ImportConfirmed,now(),&hub) {eprintln!("import receipt: {error}");}
                                    if let Some(current_receipt)=t.current(){receipt=current_receipt.clone();}
                                }
                        let proof=verify_prepared(&current.cfg,&receipt,&cached.as_ref().expect("prepared write").1).await;
                        if changes.borrow().digest!=current.digest {continue}
                        match proof {
                            Ok(Proof::Verified(proof))=>{
                                if let Ok(mut t)=tracker.try_lock()
                                    && t.current().is_some_and(|r|r.write_id==receipt.write_id )
                                        && let Err(error)=t.accept(&proof,now(),&hub) {eprintln!("apply verification receipt: {error}");}
                            }
                            Ok(Proof::Waiting(w))=>{
                                if let Ok(t)=tracker.try_lock()
                                    && t.current().is_some_and(|r|r.write_id==receipt.write_id && r.phase!=Phase::Applied) {
                                    let _=hub.observe(Observation{domain:Domain::Apply,component:"take_effect".into(),state:State::Updating,
                                        reason:format!("waiting_to_take_effect:{}",w.waiting.len()+w.waiting_removal.len()),
                                        observed_at:now(),max_age:crate::runner::IMPORT_LIMIT_SECS,source_epoch:receipt.write_id.clone(),last_success:None});
                                }
                                timeout(&tracker,&receipt,&hub)
                            }
                            Err(_)=>timeout(&tracker,&receipt,&hub),
                        }
                    }
                }
            }
        });
        Self { settings, task }
    }
    pub fn reconfigure(&self, cfg: &Config, digest: &str) {
        self.settings.send_replace(Settings {
            cfg: cfg.clone(),
            digest: digest.into(),
        });
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.task.abort();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        apply_evidence::verify::tests::{live, state},
        write_receipt::Meta,
    };
    use serde_json::json;
    /// A tracker with one receipt for the pair that `live` wrote.
    fn tracked(
        cfg: &Config,
        written: &[(std::path::PathBuf, Vec<u8>)],
        hub: &Hub,
    ) -> (tempfile::TempDir, Arc<Mutex<Tracker>>) {
        let dir = tempfile::tempdir().unwrap();
        let mut tracker = Tracker::load(dir.path()).unwrap();
        let files: Vec<_> = written
            .iter()
            .map(|(p, b)| (p.clone(), String::from_utf8(b.clone()).unwrap()))
            .collect();
        let meta = Meta {
            topology: None,
            cycle_seq: 1,
            config_digest: "cfg".into(),
            notice: None,
        };
        tracker.write(&files, meta, now(), hub);
        let _ = cfg;
        (dir, Arc::new(Mutex::new(tracker)))
    }
    async fn until(tracker: &Mutex<Tracker>, want: impl Fn(Phase) -> bool) -> Phase {
        for _ in 0..40 {
            let phase = tracker.lock().unwrap().current().unwrap().phase;
            if want(phase) {
                return phase;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        tracker.lock().unwrap().current().unwrap().phase
    }
    #[tokio::test]
    async fn a_waiting_world_reports_its_count_and_keeps_the_receipt_pending() {
        let (_d, cfg, written) = live(|d| {
            let mut other = d.clone();
            other["leaf_ops"][0]["download"]["ceil_kbps"] = json!(1);
            state("old", &other)
        });
        let hub = Hub::default();
        let (_t, tracker) = tracked(&cfg, &written, &hub);
        let _monitor = Monitor::start(&cfg, "d", tracker.clone(), hub.clone());
        assert_eq!(
            until(&tracker, |p| p == Phase::ImportConfirmed).await,
            Phase::ImportConfirmed
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
        let c = hub.snapshot(now(), None).components;
        let t = c.iter().find(|c| c.component == "take_effect").unwrap();
        assert_eq!(t.reason, "waiting_to_take_effect:1");
        assert_eq!(
            tracker.lock().unwrap().current().unwrap().phase,
            Phase::ImportConfirmed
        );
    }
    #[tokio::test]
    async fn a_late_native_apply_recovers_a_timed_out_receipt() {
        let (_d, cfg, written) = live(|d| state("g1", d));
        let hub = Hub::default();
        let (_t, tracker) = tracked(&cfg, &written, &hub);
        tracker
            .lock()
            .unwrap()
            .transition(Phase::VerifyTimeout, now(), &hub)
            .unwrap();
        let _monitor = Monitor::start(&cfg, "d", tracker.clone(), hub.clone());
        assert_eq!(
            until(&tracker, |p| p == Phase::Applied).await,
            Phase::Applied
        );
    }
}
