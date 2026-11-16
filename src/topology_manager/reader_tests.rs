use super::*;
use std::collections::VecDeque;

struct Reply {
    table: Table,
    delay: Duration,
    result: Result<Vec<RosRow>, ReadError>,
}

#[derive(Default)]
struct Session {
    replies: VecDeque<Reply>,
    calls: Vec<Table>,
}

impl RouterSession for Session {
    async fn print(&mut self, path: &str, fields: &str) -> Result<Vec<RosRow>, ReadError> {
        let reply = self.replies.pop_front().expect("unexpected command");
        assert_eq!(reply.table.command(), (path, fields));
        self.calls.push(reply.table);
        tokio::time::sleep(reply.delay).await;
        reply.result
    }
}

fn row(id: &str, name: &str) -> RosRow {
    [(".id".into(), id.into()), ("name".into(), name.into())].into()
}

fn plan(roles: &[RouterRole]) -> ReadPlan {
    let mut catalog = super::super::catalog::tests::catalog();
    let router_id = catalog.routers[0].id.clone();
    catalog.routers[0].roles = roles.to_vec();
    // This fixture tests the reader's role selection independently of source definitions.
    catalog.sources.clear();
    catalog.nodes.clear();
    ReadPlan::for_router(&catalog, &router_id).unwrap()
}

fn complete(plan: &ReadPlan) -> Session {
    let replies = plan
        .inventory
        .iter()
        .chain(plan.active.iter())
        .chain(plan.inventory.iter())
        .map(|&table| Reply {
            table,
            delay: Duration::ZERO,
            result: Ok(Vec::new()),
        })
        .collect();
    Session {
        replies,
        calls: vec![],
    }
}

async fn read(
    guard: &mut ReadGuard,
    session: &mut Session,
    plan: &ReadPlan,
    generation: u64,
) -> Result<RouterRead, ReadError> {
    guard
        .read_current(
            session,
            plan,
            generation,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
        .await
}

#[test]
fn role_plans_request_only_required_tables_and_never_passwords_or_configured_dhcp_endpoints() {
    for (role, inventory, active) in [
        (RouterRole::Wan, vec![Table::Interfaces], vec![]),
        (
            RouterRole::Ppp,
            vec![Table::PppSecrets, Table::PppProfiles],
            vec![Table::PppActive],
        ),
        (
            RouterRole::Dhcp,
            vec![Table::DhcpServers],
            vec![Table::DhcpLeases],
        ),
        (
            RouterRole::Hotspot,
            vec![Table::HotspotUsers, Table::HotspotProfiles],
            vec![Table::HotspotActive],
        ),
    ] {
        let p = plan(&[role]);
        assert_eq!(p.inventory, inventory.into_iter().collect());
        assert_eq!(p.active, active.into_iter().collect());
        for table in p.inventory.iter().chain(&p.active) {
            let (_, fields) = table.command();
            assert!(fields.split(',').all(|f| f != "password"));
            assert!(fields.split(',').any(|f| f == ".id"));
        }
    }
    let fields: BTreeSet<_> = Table::DhcpLeases.command().1.split(',').collect();
    assert!(!fields.contains("address"));
    assert!(!fields.contains("mac-address"));
    assert!(fields.contains("active-address") && fields.contains("active-mac-address"));
}

#[tokio::test(start_paused = true)]
async fn all_roles_complete_once_with_inventory_checks_and_empty_presence() {
    let p = plan(&[
        RouterRole::Wan,
        RouterRole::Ppp,
        RouterRole::Dhcp,
        RouterRole::Hotspot,
    ]);
    let mut s = complete(&p);
    let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
    let evidence = read(&mut guard, &mut s, &p, 1).await.unwrap();
    assert_eq!(evidence.generation(), 1);
    assert_eq!(evidence.router_id(), p.router_id);
    assert_eq!(s.calls.len(), 15);
    assert!(s.replies.is_empty());
    for table in p.inventory.iter().chain(&p.active) {
        assert!(evidence.rows(*table).unwrap().is_empty());
    }
}

#[tokio::test(start_paused = true)]
async fn changed_inventory_rejects_the_entire_read_but_reply_order_does_not() {
    let p = plan(&[RouterRole::Wan]);
    for changed in [false, true] {
        let mut s = complete(&p);
        s.replies[0].result = Ok(vec![row("*1", "wan1"), row("*2", "wan2")]);
        s.replies[1].result = Ok(vec![
            row("*2", "wan2"),
            row("*1", if changed { "renamed" } else { "wan1" }),
        ]);
        let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
        assert_eq!(read(&mut guard, &mut s, &p, 1).await.is_err(), changed);
    }
}

#[tokio::test(start_paused = true)]
async fn missing_duplicate_or_unrequested_fields_fail_without_row_values_in_errors() {
    let p = plan(&[RouterRole::Wan]);
    for rows in [
        vec![[("name".into(), "private-customer".into())].into()],
        vec![row("*1", "private-customer"), row("*1", "private-other")],
        vec![row("", "private-customer")],
        vec![
            [
                (".id".into(), "*1".into()),
                ("password".into(), "private-password".into()),
            ]
            .into(),
        ],
    ] {
        let mut s = complete(&p);
        s.replies[0].result = Ok(rows);
        let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
        let error = read(&mut guard, &mut s, &p, 1)
            .await
            .err()
            .unwrap()
            .to_string();
        assert!(!error.contains("private-"));
    }
}

#[tokio::test(start_paused = true)]
async fn failures_and_cancellation_consume_generations_without_complete_evidence() {
    let p = plan(&[RouterRole::Ppp]);
    let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
    let mut s = complete(&p);
    s.replies[2].result = Err(ReadError::Trap("read failed".into()));
    assert!(read(&mut guard, &mut s, &p, 4).await.is_err());
    let calls = s.calls.len();
    for generation in [0, 1, 4] {
        assert!(read(&mut guard, &mut s, &p, generation).await.is_err());
    }
    assert_eq!(calls, s.calls.len());
    let mut s = complete(&p);
    s.replies[0].delay = Duration::from_secs(5);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), read(&mut guard, &mut s, &p, 5))
            .await
            .is_err()
    );
    assert!(read(&mut guard, &mut complete(&p), &p, 5).await.is_err());
    assert!(read(&mut guard, &mut complete(&p), &p, 6).await.is_ok());
}

#[tokio::test(start_paused = true)]
async fn each_command_timeout_and_whole_read_age_are_enforced() {
    let p = plan(&[RouterRole::Wan]);
    for (delay, command_limit, age) in [(2, 1, 10), (2, 5, 3)] {
        let mut s = complete(&p);
        for r in &mut s.replies {
            r.delay = Duration::from_secs(delay);
        }
        let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
        assert!(matches!(
            guard
                .read_current(
                    &mut s,
                    &p,
                    1,
                    Duration::from_secs(command_limit),
                    Duration::from_secs(age)
                )
                .await,
            Err(ReadError::Timeout(_))
        ));
    }
}

#[tokio::test(start_paused = true)]
async fn evidence_expires_from_read_start_and_unread_tables_are_not_empty_successes() {
    let p = plan(&[RouterRole::Wan]);
    let mut s = complete(&p);
    for r in &mut s.replies {
        r.delay = Duration::from_secs(2);
    }
    let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
    let evidence = guard
        .read_current(
            &mut s,
            &p,
            1,
            Duration::from_secs(5),
            Duration::from_secs(6),
        )
        .await
        .unwrap();
    assert!(evidence.rows(Table::Interfaces).is_ok());
    assert!(evidence.rows(Table::PppActive).is_err());
    tokio::time::advance(Duration::from_secs(2)).await;
    assert!(matches!(
        evidence.rows(Table::Interfaces),
        Err(ReadError::Timeout(_))
    ));
}

#[tokio::test(start_paused = true)]
async fn wrong_router_or_zero_limits_cannot_issue_commands() {
    let p = plan(&[RouterRole::Wan]);
    let mut guard = ReadGuard::new("other-router".into()).unwrap();
    let mut s = complete(&p);
    assert!(read(&mut guard, &mut s, &p, 1).await.is_err());
    let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
    for (generation, command, age) in [
        (1, Duration::ZERO, Duration::from_secs(5)),
        (2, Duration::from_secs(5), Duration::ZERO),
    ] {
        assert!(
            guard
                .read_current(&mut s, &p, generation, command, age)
                .await
                .is_err()
        );
    }
    assert!(s.calls.is_empty());
    assert!(ReadGuard::new(" ".into()).is_err());
}

#[test]
fn unknown_router_and_invalid_catalog_cannot_create_a_plan() {
    let mut catalog = super::super::catalog::tests::catalog();
    assert!(ReadPlan::for_router(&catalog, "absent-router").is_err());
    let role = catalog.routers[0].roles[0];
    catalog.routers[0].roles.push(role);
    assert!(ReadPlan::for_router(&catalog, &catalog.routers[0].id).is_err());
}

#[tokio::test(start_paused = true)]
async fn public_reader_returns_only_successful_sessions_and_discards_failed_or_cancelled_ones() {
    let p = plan(&[RouterRole::Wan]);
    let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
    let mut slot = Some(complete(&p));
    let limit = Duration::from_secs(10);
    let evidence = guard.read(&mut slot, &p, 1, limit, limit).await.unwrap();
    assert!(evidence.rows(Table::Interfaces).is_ok());
    assert!(slot.as_ref().unwrap().replies.is_empty());
    slot = Some(complete(&p));
    slot.as_mut().unwrap().replies[0].result = Err(ReadError::Trap("failed".into()));
    assert!(guard.read(&mut slot, &p, 2, limit, limit).await.is_err());
    assert!(slot.is_none());
    slot = Some(complete(&p));
    slot.as_mut().unwrap().replies[0].delay = Duration::from_secs(5);
    assert!(
        tokio::time::timeout(
            Duration::from_secs(1),
            guard.read(&mut slot, &p, 3, limit, limit)
        )
        .await
        .is_err()
    );
    assert!(slot.is_none());
    slot = Some(complete(&p));
    assert!(guard.read(&mut slot, &p, 3, limit, limit).await.is_err());
    assert!(slot.is_none());
    slot = Some(complete(&p));
    assert!(guard.read(&mut slot, &p, 4, limit, limit).await.is_ok());
    assert!(slot.is_some());
}

#[tokio::test(start_paused = true)]
async fn large_inventory_is_compared_by_locator_and_keeps_current_rows_only() {
    let p = plan(&[RouterRole::Wan]);
    let mut session = complete(&p);
    let rows: Vec<_> = (0..10_000)
        .map(|i| row(&format!("*{i:x}"), &format!("ether{i}")))
        .collect();
    session.replies[0].result = Ok(rows.clone());
    session.replies[1].result = Ok(rows.into_iter().rev().collect());
    let mut guard = ReadGuard::new(p.router_id.clone()).unwrap();
    let mut slot = Some(session);
    let evidence = guard
        .read(
            &mut slot,
            &p,
            1,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
        .await
        .unwrap();
    assert_eq!(evidence.rows(Table::Interfaces).unwrap().len(), 10_000);
}
