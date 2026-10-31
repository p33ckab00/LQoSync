//! RouterOS reads: the error type, the session traits, and the snapshot read.

use crate::config::{RouterAccess, RouterCfg};
use crate::model::{RosRow, RouterSnapshot};
use std::fmt;
use std::time::Duration;

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

/// One open API session to one router. It only reads.
pub trait RouterSession: Send {
    /// Send `<path>/print` with `=.proplist=<proplist>` and return all rows.
    fn print(
        &mut self,
        path: &str,
        proplist: &str,
    ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send;
}

/// Opens sessions to routers.
pub trait Connector: Send + Sync {
    type Session: RouterSession;
    fn connect(
        &self,
        router: &RouterCfg,
        access: &RouterAccess,
    ) -> impl Future<Output = Result<Self::Session, ReadError>> + Send;
}

/// The tables and their fields, in read order (spec section 3).
pub const TABLES: [(&str, &str); 6] = [
    ("/ppp/secret", "name,profile,comment,disabled,caller-id"),
    ("/ppp/active", "name,address,caller-id,comment"),
    ("/ppp/profile", "name,comment,rate-limit"),
    ("/ip/hotspot/active", "user,address,mac-address"),
    ("/ip/dhcp-server", "name,comment,disabled"),
    (
        "/ip/dhcp-server/lease",
        "server,address,active-address,mac-address,host-name,comment,status",
    ),
];

/// Read the tables of the enabled sources. Each call has `api_timeout`.
pub async fn read_snapshot<S: RouterSession>(
    s: &mut S,
    router: &RouterCfg,
    api_timeout: Duration,
) -> Result<RouterSnapshot, ReadError> {
    let mut snap = RouterSnapshot::default();
    let wanted = [
        router.pppoe.enabled,
        router.pppoe.enabled,
        router.pppoe.enabled,
        router.hotspot.enabled,
        router.dhcp.enabled,
        router.dhcp.enabled,
    ];
    for (i, (path, fields)) in TABLES.iter().enumerate() {
        if !wanted[i] {
            continue;
        }
        let rows = tokio::time::timeout(api_timeout, s.print(path, fields))
            .await
            .map_err(|_| {
                ReadError::Timeout(format!(
                    "{path}/print took more than {} s",
                    api_timeout.as_secs()
                ))
            })??;
        let slot = match i {
            0 => &mut snap.ppp_secret,
            1 => &mut snap.ppp_active,
            2 => &mut snap.ppp_profile,
            3 => &mut snap.hotspot_active,
            4 => &mut snap.dhcp_server,
            _ => &mut snap.dhcp_lease,
        };
        *slot = rows;
    }
    Ok(snap)
}

/// Read one router with retries. A failed attempt closes the session; the next one connects again.
/// A login error is not retried: the same password gives the same answer.
pub async fn read_router<C: Connector>(
    c: &C,
    session: &mut Option<C::Session>,
    router: &RouterCfg,
    access: &RouterAccess,
) -> Result<RouterSnapshot, ReadError> {
    let attempts = 1 + access.read_retries;
    let mut last = ReadError::Protocol("no attempt".into());
    // mikrotik-tokio 0.3 has no TCP keepalive, so a kept session can be dead after an idle time.
    // Its failure gets one reconnect at once, which does not use a retry.
    let mut free_reconnect = session.is_some();
    let mut n = 0;
    while n < attempts {
        if n > 0 {
            tokio::time::sleep(Duration::from_secs(access.retry_delay)).await;
        }
        if session.is_none() {
            // The connector reports a failed TCP connect as Unreachable. A login or TLS handshake
            // that hangs after that is a Timeout, so the router keeps its rows.
            let limit = Duration::from_secs(access.connect_timeout + access.api_timeout);
            let connect = tokio::time::timeout(limit, c.connect(router, access));
            match connect.await {
                Ok(Ok(s)) => *session = Some(s),
                Ok(Err(e @ ReadError::Login(_))) => return Err(e),
                Ok(Err(e)) => {
                    last = e;
                    n += 1;
                    continue;
                }
                Err(_) => {
                    last = ReadError::Timeout(format!(
                        "connect and login took more than {} s",
                        limit.as_secs()
                    ));
                    n += 1;
                    continue;
                }
            }
        }
        let s = session.as_mut().expect("session was set above");
        match read_snapshot(s, router, Duration::from_secs(access.api_timeout)).await {
            Ok(snap) => return Ok(snap),
            Err(e) => {
                *session = None;
                last = e;
                if std::mem::take(&mut free_reconnect) {
                    continue;
                }
            }
        }
        n += 1;
    }
    Err(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, RouterAccess, RouterCfg};
    use crate::model::RosRow;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    type Script = Arc<Mutex<VecDeque<Result<Vec<RosRow>, ReadError>>>>;

    #[derive(Clone, Default)]
    struct Fake {
        calls: Arc<Mutex<Vec<String>>>,
        script: Script,
        connects: Arc<AtomicUsize>,
        stall: Option<Duration>,
        stall_connect: bool,
        connect_err: Option<ReadError>,
    }
    struct FakeSession(Fake);

    impl Connector for Fake {
        type Session = FakeSession;
        fn connect(
            &self,
            _r: &RouterCfg,
            _a: &RouterAccess,
        ) -> impl Future<Output = Result<FakeSession, ReadError>> + Send {
            self.connects.fetch_add(1, Ordering::SeqCst);
            let me = self.clone();
            async move {
                if me.stall_connect {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                }
                if let Some(e) = me.connect_err.clone() {
                    return Err(e);
                }
                Ok(FakeSession(me))
            }
        }
    }
    impl RouterSession for FakeSession {
        fn print(
            &mut self,
            path: &str,
            proplist: &str,
        ) -> impl Future<Output = Result<Vec<RosRow>, ReadError>> + Send {
            let f = self.0.clone();
            let call = format!("{path} {proplist}");
            async move {
                f.calls.lock().unwrap().push(call);
                if let Some(d) = f.stall {
                    tokio::time::sleep(d).await;
                }
                f.script.lock().unwrap().pop_front().unwrap_or(Ok(vec![]))
            }
        }
    }

    fn ppp_router() -> (RouterCfg, RouterAccess) {
        let c = Config::from_toml_str(
            "[[routers]]\nname=\"R1\"\naddress=\"192.0.2.1\"\nusername=\"u\"\npassword_file=\"/x\"\n[routers.pppoe]\nenabled=true\n",
        )
        .unwrap();
        (c.routers[0].clone(), c.router_access.clone())
    }
    fn row(k: &str, v: &str) -> RosRow {
        [(k.to_string(), v.to_string())].into_iter().collect()
    }

    #[tokio::test(start_paused = true)]
    async fn reads_only_enabled_tables() {
        let (r, a) = ppp_router();
        let f = Fake::default();
        f.script
            .lock()
            .unwrap()
            .extend([Ok(vec![row("name", "alice")]), Ok(vec![]), Ok(vec![])]);
        let mut s = None;
        let snap = read_router(&f, &mut s, &r, &a).await.unwrap();
        assert_eq!(snap.ppp_secret, vec![row("name", "alice")]);
        let calls = f.calls.lock().unwrap().clone();
        assert_eq!(
            calls,
            [
                format!("/ppp/secret {}", TABLES[0].1),
                format!("/ppp/active {}", TABLES[1].1),
                format!("/ppp/profile {}", TABLES[2].1),
            ]
        );
    }

    #[tokio::test(start_paused = true)]
    async fn retry_reconnects_and_succeeds() {
        let (r, a) = ppp_router();
        let f = Fake::default();
        f.script
            .lock()
            .unwrap()
            .push_back(Err(ReadError::Trap("busy".into())));
        let mut s = None;
        assert!(read_router(&f, &mut s, &r, &a).await.is_ok());
        assert_eq!(f.connects.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn gives_last_error_after_all_retries() {
        let (r, a) = ppp_router();
        let f = Fake::default();
        for i in 0..3 {
            f.script
                .lock()
                .unwrap()
                .push_back(Err(ReadError::Trap(format!("t{i}"))));
        }
        let mut s = None;
        assert_eq!(
            read_router(&f, &mut s, &r, &a).await.unwrap_err(),
            ReadError::Trap("t2".into())
        );
        assert_eq!(f.connects.load(Ordering::SeqCst), 3);
        assert!(s.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn stalled_call_times_out() {
        let (r, a) = ppp_router();
        let f = Fake {
            stall: Some(Duration::from_secs(3600)),
            ..Fake::default()
        };
        let mut s = None;
        assert!(matches!(
            read_router(&f, &mut s, &r, &a).await,
            Err(ReadError::Timeout(_))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn session_is_kept_between_reads() {
        let (r, a) = ppp_router();
        let f = Fake::default();
        let mut s = None;
        read_router(&f, &mut s, &r, &a).await.unwrap();
        read_router(&f, &mut s, &r, &a).await.unwrap();
        assert_eq!(f.connects.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn stalled_connect_is_a_timeout_not_unreachable() {
        let (r, a) = ppp_router();
        let f = Fake {
            stall_connect: true,
            ..Fake::default()
        };
        let mut s = None;
        assert!(matches!(
            read_router(&f, &mut s, &r, &a).await,
            Err(ReadError::Timeout(_))
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn a_login_error_is_not_retried() {
        let (r, a) = ppp_router();
        let f = Fake {
            connect_err: Some(ReadError::Login("bad password file".into())),
            ..Fake::default()
        };
        let mut s = None;
        assert!(matches!(
            read_router(&f, &mut s, &r, &a).await,
            Err(ReadError::Login(_))
        ));
        assert_eq!(f.connects.load(Ordering::SeqCst), 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_dead_kept_session_reconnects_without_a_retry() {
        let (r, mut a) = ppp_router();
        a.read_retries = 0;
        let f = Fake::default();
        let mut s = None;
        read_router(&f, &mut s, &r, &a).await.unwrap();
        // The kept session is dead: the first call of the next cycle fails.
        f.script
            .lock()
            .unwrap()
            .push_back(Err(ReadError::Timeout("dead".into())));
        read_router(&f, &mut s, &r, &a).await.unwrap();
        assert_eq!(f.connects.load(Ordering::SeqCst), 2);
    }
}
