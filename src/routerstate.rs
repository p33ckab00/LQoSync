//! Router state: the outcome of the reads of one cycle to a `RouterResult` (spec section 4).

use crate::config::RouterAccess;
use crate::cycle::RouterResult;
use crate::model::{RouterMemo, RouterSnapshot};
use crate::routeros::ReadError;

/// Classify the reads of one router. Returns the result for the core, the new memo,
/// and the error text for the status.
pub fn classify(
    outcome: Result<RouterSnapshot, ReadError>,
    memo: &RouterMemo,
    now: u64,
    access: &RouterAccess,
) -> (RouterResult, RouterMemo, Option<String>) {
    match outcome {
        Ok(snap) => (
            RouterResult::Online(snap),
            RouterMemo {
                unreachable_since: None,
            },
            None,
        ),
        Err(e @ ReadError::Unreachable(_)) => {
            let since = memo.unreachable_since.unwrap_or(now);
            let result = if now.saturating_sub(since) >= access.offline_after {
                RouterResult::Offline
            } else {
                RouterResult::Unreachable
            };
            (
                result,
                RouterMemo {
                    unreachable_since: Some(since),
                },
                Some(e.to_string()),
            )
        }
        // A config or protocol problem: the router is there, so it keeps its rows.
        Err(e) => (RouterResult::Failed, memo.clone(), Some(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RouterAccess;
    use crate::cycle::RouterResult;
    use crate::model::{RouterMemo, RouterSnapshot};
    use crate::routeros::ReadError;

    fn acc(offline_after: u64) -> RouterAccess {
        RouterAccess {
            offline_after,
            ..RouterAccess::default()
        }
    }
    fn memo(since: Option<u64>) -> RouterMemo {
        RouterMemo {
            unreachable_since: since,
        }
    }
    fn name(r: &RouterResult) -> &'static str {
        match r {
            RouterResult::Online(_) => "online",
            RouterResult::Failed => "failed",
            RouterResult::Unreachable => "unreachable",
            RouterResult::Offline => "offline",
        }
    }

    #[test]
    fn ok_is_online_and_clears_memo() {
        let (r, m, e) = classify(
            Ok(RouterSnapshot::default()),
            &memo(Some(5)),
            100,
            &acc(300),
        );
        assert_eq!((name(&r), m, e), ("online", memo(None), None));
    }

    #[test]
    fn login_tls_trap_timeout_are_failed_and_keep_memo() {
        for err in [
            ReadError::Login("x".into()),
            ReadError::Tls("x".into()),
            ReadError::Trap("x".into()),
            ReadError::Timeout("x".into()),
            ReadError::Protocol("x".into()),
        ] {
            let (r, m, e) = classify(Err(err.clone()), &memo(Some(7)), 100, &acc(300));
            assert_eq!(
                (name(&r), m, e),
                ("failed", memo(Some(7)), Some(err.to_string()))
            );
        }
    }

    #[test]
    fn first_unreachable_sets_since() {
        let (r, m, _) = classify(
            Err(ReadError::Unreachable("refused".into())),
            &memo(None),
            100,
            &acc(300),
        );
        assert_eq!((name(&r), m), ("unreachable", memo(Some(100))));
    }

    #[test]
    fn unreachable_before_offline_after() {
        let (r, m, _) = classify(
            Err(ReadError::Unreachable("x".into())),
            &memo(Some(1000)),
            1299,
            &acc(300),
        );
        assert_eq!((name(&r), m), ("unreachable", memo(Some(1000))));
    }

    #[test]
    fn offline_at_offline_after() {
        let (r, m, _) = classify(
            Err(ReadError::Unreachable("x".into())),
            &memo(Some(700)),
            1000,
            &acc(300),
        );
        assert_eq!((name(&r), m), ("offline", memo(Some(700))));
    }

    #[test]
    fn offline_after_zero_is_offline_at_once() {
        let (r, _, _) = classify(
            Err(ReadError::Unreachable("x".into())),
            &memo(None),
            1000,
            &acc(0),
        );
        assert_eq!(name(&r), "offline");
    }
}
