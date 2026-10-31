//! Manual RouterOS acceptance, Phase 1: one test host on one core router.
//! See docs/superpowers/specs/2026-10-31-routeros-acceptance-phase1-design.md.
use anyhow::{Context, Result, ensure};
use lqosync_v3::{core_migration::Member, routeros::Row};
use std::collections::BTreeMap;

const TEST_ADDRESS: &str = "192.0.2.254";
const TEST_SUBSCRIBER: &str = "acceptance-test";

fn main() -> Result<()> {
    anyhow::bail!("Usage: core-acceptance [--execute]")
}

fn by_id(rows: &[Member]) -> BTreeMap<&str, &Member> {
    rows.iter().map(|r| (r.id.as_str(), r)).collect()
}
fn others<'a>(rows: &'a [Member], id: &str) -> BTreeMap<&'a str, &'a Member> {
    let mut map = by_id(rows);
    map.remove(id);
    map
}
fn find<'a>(rows: &'a [Member], id: &str) -> Result<&'a Member> {
    rows.iter()
        .find(|r| r.id == id)
        .with_context(|| format!("Test row {id} is missing"))
}
/// Stop before any write if the test host or its marker already exists.
fn check_clear(rows: &[Member]) -> Result<()> {
    let marker = format!(":{TEST_SUBSCRIBER}:");
    for r in rows {
        ensure!(
            r.address.strip_suffix("/32").unwrap_or(&r.address) != TEST_ADDRESS,
            "Test address already exists as {}",
            r.id
        );
        ensure!(
            !r.comment.contains(&marker),
            "Test marker already exists on {}",
            r.id
        );
    }
    Ok(())
}
fn check_added(before: &[Member], after: &[Member], id: &str, expected: &Member) -> Result<()> {
    ensure!(
        !by_id(before).contains_key(id),
        "Returned ID {id} existed before the add"
    );
    ensure!(
        others(after, id) == by_id(before),
        "Other address-list rows changed during the add"
    );
    let row = find(after, id)?;
    let want = Member {
        id: id.into(),
        ..expected.clone()
    };
    ensure!(*row == want, "New row fields differ: {row:?}");
    Ok(())
}
fn check_moved(
    before: &[Member],
    after: &[Member],
    id: &str,
    list: &str,
    comment: &str,
) -> Result<()> {
    ensure!(
        others(after, id) == others(before, id),
        "Other address-list rows changed during the move"
    );
    let old = find(before, id)?;
    let want = Member {
        list: list.into(),
        comment: comment.into(),
        ..old.clone()
    };
    ensure!(
        *find(after, id)? == want,
        "Move changed more than list and comment"
    );
    Ok(())
}
fn check_same(before: &[Member], after: &[Member]) -> Result<()> {
    ensure!(
        by_id(before) == by_id(after),
        "Address-list rows differ from the baseline"
    );
    Ok(())
}
fn strip_counters(rows: &[Row]) -> Vec<Row> {
    rows.iter()
        .map(|r| {
            r.iter()
                .filter(|(k, _)| !matches!(k.as_str(), "bytes" | "packets"))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, list: &str, address: &str, comment: &str) -> Member {
        Member {
            id: id.into(),
            list: list.into(),
            address: address.into(),
            comment: comment.into(),
            dynamic: false,
            disabled: false,
            timeout: None,
        }
    }
    fn base() -> Vec<Member> {
        vec![m("*A", "WAN1-CUSTOMERS", "10.15.0.0/24", "IPoE vlan10")]
    }
    const W1: &str = "lqosync3:o:acceptance-test:wan1";
    const W2: &str = "lqosync3:o:acceptance-test:wan2";

    #[test]
    fn clear_rejects_test_address() {
        assert!(check_clear(&[m("*1", "X", "192.0.2.254", "")]).is_err());
        assert!(check_clear(&[m("*1", "X", "192.0.2.254/32", "")]).is_err());
    }
    #[test]
    fn clear_rejects_test_marker() {
        assert!(check_clear(&[m("*1", "X", "10.0.0.9", W1)]).is_err());
    }
    #[test]
    fn clear_accepts_other_rows() {
        assert!(check_clear(&base()).is_ok());
    }
    #[test]
    fn added_accepts_one_new_row() {
        let new = m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1);
        let after = [base(), vec![new.clone()]].concat();
        assert!(
            check_added(
                &base(),
                &after,
                "*20",
                &m("", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)
            )
            .is_ok()
        );
        assert!(check_added(&base(), &after, "*21", &new).is_err());
    }
    #[test]
    fn added_rejects_changed_other_row() {
        let mut after = base();
        after[0].comment = "changed".into();
        after.push(m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1));
        assert!(
            check_added(
                &base(),
                &after,
                "*20",
                &m("", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)
            )
            .is_err()
        );
    }
    #[test]
    fn added_rejects_wrong_fields() {
        let after = [base(), vec![m("*20", "WAN2-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        assert!(
            check_added(
                &base(),
                &after,
                "*20",
                &m("", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)
            )
            .is_err()
        );
    }
    #[test]
    fn moved_accepts_list_and_comment_change() {
        let before = [base(), vec![m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        let after = [base(), vec![m("*20", "WAN2-CUSTOMERS", TEST_ADDRESS, W2)]].concat();
        assert!(check_moved(&before, &after, "*20", "WAN2-CUSTOMERS", W2).is_ok());
    }
    #[test]
    fn moved_rejects_new_id() {
        let before = [base(), vec![m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        let after = [base(), vec![m("*21", "WAN2-CUSTOMERS", TEST_ADDRESS, W2)]].concat();
        assert!(check_moved(&before, &after, "*20", "WAN2-CUSTOMERS", W2).is_err());
    }
    #[test]
    fn moved_rejects_changed_address() {
        let before = [base(), vec![m("*20", "WAN1-CUSTOMERS", TEST_ADDRESS, W1)]].concat();
        let after = [base(), vec![m("*20", "WAN2-CUSTOMERS", "192.0.2.253", W2)]].concat();
        assert!(check_moved(&before, &after, "*20", "WAN2-CUSTOMERS", W2).is_err());
    }
    #[test]
    fn same_detects_any_change() {
        assert!(check_same(&base(), &base()).is_ok());
        let mut after = base();
        after[0].disabled = true;
        assert!(check_same(&base(), &after).is_err());
    }
    #[test]
    fn strip_counters_ignores_bytes_and_packets() {
        let row = |b: &str| -> Row {
            [("chain", "output"), ("bytes", b), ("packets", b)]
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect()
        };
        assert_eq!(strip_counters(&[row("1")]), strip_counters(&[row("9")]));
        assert_eq!(strip_counters(&[row("1")])[0].len(), 1);
    }
}
