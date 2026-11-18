use super::super::catalog::{CatalogRouter, RouterRole, TopologyCatalog};
use super::super::reader::{ReadGuard, ReadPlan};
use super::*;
use crate::model::RosRow;
use crate::routeros::RouterSession;
use std::collections::BTreeMap;
use std::time::Duration;

type Rows = BTreeMap<Table, Vec<RosRow>>;
struct Session(Rows);
impl RouterSession for Session {
    async fn print(&mut self, path: &str, fields: &str) -> Result<Vec<RosRow>, ReadError> {
        let table = [
            Table::Interfaces,
            Table::PppSecrets,
            Table::PppProfiles,
            Table::PppActive,
            Table::DhcpServers,
            Table::DhcpLeases,
            Table::HotspotUsers,
            Table::HotspotServers,
            Table::HotspotProfiles,
            Table::HotspotActive,
        ]
        .into_iter()
        .find(|t| t.command() == (path, fields))
        .unwrap();
        Ok(self.0.get(&table).cloned().unwrap_or_default())
    }
}
fn row(values: &[(&str, &str)]) -> RosRow {
    values.iter().map(|&(k, v)| (k.into(), v.into())).collect()
}
async fn evidence(rows: Rows, roles: &[RouterRole]) -> RouterRead {
    let catalog = TopologyCatalog {
        routers: vec![CatalogRouter {
            id: "router-a".into(),
            name: "Router".into(),
            roles: roles.into(),
        }],
        ..Default::default()
    };
    let plan = ReadPlan::for_router(&catalog, "router-a").unwrap();
    ReadGuard::new("router-a".into())
        .unwrap()
        .read(
            &mut Some(Session(rows)),
            &plan,
            1,
            Duration::from_secs(1),
            Duration::from_secs(10),
        )
        .await
        .unwrap()
}
fn ppp() -> Rows {
    BTreeMap::from([
        (
            Table::PppProfiles,
            vec![row(&[
                (".id", "*p"),
                ("name", "gold"),
                ("rate-limit", "2M/10M"),
            ])],
        ),
        (
            Table::PppSecrets,
            vec![row(&[
                (".id", "*s"),
                ("name", "alice"),
                ("profile", "gold"),
                ("disabled", "false"),
                ("comment", "account note"),
            ])],
        ),
        (
            Table::PppActive,
            vec![row(&[
                (".id", "*a"),
                ("name", "alice"),
                ("address", "192.0.2.1"),
                ("caller-id", "aa:bb:cc:dd:ee:01"),
                ("service", "pppoe"),
            ])],
        ),
    ])
}
fn dhcp() -> Rows {
    BTreeMap::from([
        (
            Table::DhcpServers,
            vec![row(&[
                (".id", "*d"),
                ("name", "lan"),
                ("disabled", "false"),
            ])],
        ),
        (
            Table::DhcpLeases,
            vec![row(&[
                (".id", "*l"),
                ("server", "lan"),
                ("status", "bound"),
                ("active-address", "192.0.2.2"),
                ("active-mac-address", "aa:bb:cc:dd:ee:02"),
                ("active-client-id", "client-1"),
                ("host-name", "device"),
                ("dynamic", "true"),
            ])],
        ),
    ])
}
fn hotspot() -> Rows {
    BTreeMap::from([
        (
            Table::HotspotServers,
            vec![row(&[
                (".id", "*hs"),
                ("name", "hotspot1"),
                ("disabled", "false"),
            ])],
        ),
        (
            Table::HotspotProfiles,
            vec![row(&[
                (".id", "*hp"),
                ("name", "wifi"),
                ("rate-limit", "1M/5M"),
            ])],
        ),
        (
            Table::HotspotUsers,
            vec![row(&[
                (".id", "*hu"),
                ("name", "alice"),
                ("profile", "wifi"),
                ("disabled", "false"),
            ])],
        ),
        (
            Table::HotspotActive,
            vec![row(&[
                (".id", "*ha"),
                ("user", "alice"),
                ("address", "192.0.2.3"),
                ("mac-address", "aa:bb:cc:dd:ee:03"),
                ("server", "hotspot1"),
                ("login-by", "http-chap"),
            ])],
        ),
    ])
}
fn issue(result: Result<CurrentRouter, NormalizeError>, expected: Issue) {
    assert!(matches!(result, Err(NormalizeError::Row { issue, .. }) if issue == expected));
}

#[tokio::test(start_paused = true)]
async fn ppp_current_sessions_join_one_local_account_and_current_profile() {
    let mut rows = ppp();
    let mut second = rows[&Table::PppActive][0].clone();
    second.insert(".id".into(), "*b".into());
    second.insert("address".into(), "192.0.2.4".into());
    second.insert("caller-id".into(), "aa:bb:cc:dd:ee:04".into());
    rows.get_mut(&Table::PppActive).unwrap().push(second);
    let read = evidence(rows, &[RouterRole::Ppp]).await;
    let normalized = normalize(&read).unwrap();
    assert_eq!(normalized.router_id(), "router-a");
    assert_eq!(normalized.generation(), 1);
    let accounts = normalized.accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].row_ref, "*s");
    assert_eq!(accounts[0].profile_ref.as_deref(), Some("*p"));
    assert_eq!(accounts[0].endpoints.len(), 2);
    assert_eq!(
        accounts[0].endpoints[0].mac.as_deref(),
        Some("AA:BB:CC:DD:EE:01")
    );
    assert_eq!(accounts[0].service_refs, vec!["pppoe"]);
    assert!(
        matches!(&accounts[0].authority, AuthorityEvidence::LocalAccount { name,.. } if name=="alice")
    );
    assert_eq!(
        normalized.profiles().unwrap()[0].rate_limit.as_deref(),
        Some("2M/10M")
    );
}

#[tokio::test(start_paused = true)]
async fn offline_ppp_accounts_and_unbound_static_leases_supply_no_endpoint() {
    let mut rows = ppp();
    rows.get_mut(&Table::PppActive).unwrap().clear();
    rows.extend(dhcp());
    let lease = &mut rows.get_mut(&Table::DhcpLeases).unwrap()[0];
    lease.insert("status".into(), "waiting".into());
    lease.insert("dynamic".into(), "false".into());
    let read = evidence(rows, &[RouterRole::Ppp, RouterRole::Dhcp]).await;
    let normalized = normalize(&read).unwrap();
    assert!(normalized.accounts().unwrap().is_empty());
    assert_eq!(normalized.profiles().unwrap().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn disabled_missing_or_ambiguous_ppp_accounts_fail_closed() {
    for expected in [
        Issue::DisabledAccount,
        Issue::MissingAccount,
        Issue::DuplicateName,
    ] {
        let mut rows = ppp();
        let secrets = rows.get_mut(&Table::PppSecrets).unwrap();
        match expected {
            Issue::DisabledAccount => {
                secrets[0].insert("disabled".into(), "true".into());
            }
            Issue::MissingAccount => secrets.clear(),
            _ => {
                let mut duplicate = secrets[0].clone();
                duplicate.insert(".id".into(), "*s2".into());
                secrets.push(duplicate);
            }
        }
        issue(
            normalize(&evidence(rows, &[RouterRole::Ppp]).await),
            expected,
        );
    }
}

#[tokio::test(start_paused = true)]
async fn duplicate_or_missing_profiles_do_not_invent_current_membership_or_rates() {
    for duplicate in [false, true] {
        let mut rows = ppp();
        let profiles = rows.get_mut(&Table::PppProfiles).unwrap();
        if duplicate {
            let mut extra = profiles[0].clone();
            extra.insert(".id".into(), "*p2".into());
            profiles.push(extra);
        } else {
            profiles.clear();
        }
        issue(
            normalize(&evidence(rows, &[RouterRole::Ppp]).await),
            if duplicate {
                Issue::DuplicateName
            } else {
                Issue::MissingProfile
            },
        );
    }
}

#[tokio::test(start_paused = true)]
async fn ppp_caller_id_can_be_non_mac_without_reusing_any_old_mac() {
    let mut rows = ppp();
    rows.get_mut(&Table::PppActive).unwrap()[0].insert("caller-id".into(), "caller-phone".into());
    let read = evidence(rows, &[RouterRole::Ppp]).await;
    assert!(
        normalize(&read).unwrap().accounts().unwrap()[0].endpoints[0]
            .mac
            .is_none()
    );
}

#[tokio::test(start_paused = true)]
async fn dhcp_bound_uses_current_fields_and_retains_endpoint_only_evidence() {
    let read = evidence(dhcp(), &[RouterRole::Dhcp]).await;
    let normalized = normalize(&read).unwrap();
    let accounts = normalized.accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].row_ref, "*l");
    assert_eq!(accounts[0].endpoints[0].ips[0].to_string(), "192.0.2.2");
    assert_eq!(accounts[0].service_refs, vec!["*d"]);
    assert!(
        matches!(&accounts[0].authority, AuthorityEvidence::EndpointOnly { name:Some(name),client_id:Some(client),.. } if name=="device" && client=="client-1")
    );
    assert!(accounts[0].profile_ref.is_none());
}

#[tokio::test(start_paused = true)]
async fn missing_active_dhcp_endpoint_or_status_cannot_be_replaced_by_retained_data() {
    for field in ["active-address", "status"] {
        let mut rows = dhcp();
        rows.get_mut(&Table::DhcpLeases).unwrap()[0].remove(field);
        issue(
            normalize(&evidence(rows, &[RouterRole::Dhcp]).await),
            Issue::MissingField,
        );
    }
}

#[tokio::test(start_paused = true)]
async fn all_documented_unbound_lease_states_are_absent_and_unknown_states_fail() {
    for status in [
        "waiting",
        "testing",
        "declined",
        "offered",
        "authorizing",
        "conflict",
        "unknown",
    ] {
        let mut rows = dhcp();
        rows.get_mut(&Table::DhcpLeases).unwrap()[0].insert("status".into(), status.into());
        let result = normalize(&evidence(rows, &[RouterRole::Dhcp]).await);
        if status == "unknown" {
            issue(result, Issue::InvalidField);
        } else {
            assert!(result.unwrap().accounts().unwrap().is_empty());
        }
    }
}

#[tokio::test(start_paused = true)]
async fn hostname_or_mac_reuse_does_not_merge_dhcp_leases() {
    let mut rows = dhcp();
    let mut extra = rows[&Table::DhcpLeases][0].clone();
    extra.insert(".id".into(), "*l2".into());
    extra.insert("active-address".into(), "192.0.2.5".into());
    rows.get_mut(&Table::DhcpLeases).unwrap().push(extra);
    let read = evidence(rows, &[RouterRole::Dhcp]).await;
    assert_eq!(normalize(&read).unwrap().accounts().unwrap().len(), 2);
}

#[tokio::test(start_paused = true)]
async fn missing_or_duplicate_dhcp_server_is_a_conflict() {
    for duplicate in [false, true] {
        let mut rows = dhcp();
        let servers = rows.get_mut(&Table::DhcpServers).unwrap();
        if duplicate {
            let mut extra = servers[0].clone();
            extra.insert(".id".into(), "*d2".into());
            servers.push(extra);
        } else {
            servers.clear();
        }
        issue(
            normalize(&evidence(rows, &[RouterRole::Dhcp]).await),
            if duplicate {
                Issue::DuplicateName
            } else {
                Issue::MissingServer
            },
        );
    }
}

#[tokio::test(start_paused = true)]
async fn hotspot_named_authentication_joins_local_account_and_profile() {
    for method in ["http-chap", "http-pap", "https", "cookie", "mac-cookie"] {
        let mut rows = hotspot();
        rows.get_mut(&Table::HotspotActive).unwrap()[0].insert("login-by".into(), method.into());
        let read = evidence(rows, &[RouterRole::Hotspot]).await;
        let normalized = normalize(&read).unwrap();
        let accounts = normalized.accounts().unwrap();
        assert!(matches!(
            accounts[0].authority,
            AuthorityEvidence::LocalAccount { .. }
        ));
        assert_eq!(accounts[0].profile_ref.as_deref(), Some("*hp"));
    }
}

#[tokio::test(start_paused = true)]
async fn hotspot_mac_and_trial_do_not_claim_stable_local_account_authority() {
    for method in ["mac", "trial"] {
        let mut rows = hotspot();
        rows.get_mut(&Table::HotspotActive).unwrap()[0].insert("login-by".into(), method.into());
        if method == "trial" {
            rows.get_mut(&Table::HotspotUsers).unwrap().clear();
        }
        let read = evidence(rows, &[RouterRole::Hotspot]).await;
        assert!(matches!(
            normalize(&read).unwrap().accounts().unwrap()[0].authority,
            AuthorityEvidence::EndpointOnly { .. }
        ));
    }
}

#[tokio::test(start_paused = true)]
async fn hotspot_unknown_authentication_or_missing_local_account_is_a_conflict() {
    for missing in [false, true] {
        let mut rows = hotspot();
        if missing {
            rows.get_mut(&Table::HotspotUsers).unwrap().clear();
        } else {
            rows.get_mut(&Table::HotspotActive).unwrap()[0]
                .insert("login-by".into(), "unknown".into());
        }
        issue(
            normalize(&evidence(rows, &[RouterRole::Hotspot]).await),
            if missing {
                Issue::MissingAccount
            } else {
                Issue::UnsupportedAuthentication
            },
        );
    }
}

#[tokio::test(start_paused = true)]
async fn identical_usernames_in_different_protocols_do_not_merge_accounts() {
    let mut rows = ppp();
    rows.extend(hotspot());
    let read = evidence(rows, &[RouterRole::Ppp, RouterRole::Hotspot]).await;
    assert_eq!(normalize(&read).unwrap().accounts().unwrap().len(), 2);
}

#[tokio::test(start_paused = true)]
async fn conflicting_current_ip_ownership_across_protocols_is_not_hidden() {
    let mut rows = ppp();
    rows.extend(dhcp());
    rows.get_mut(&Table::DhcpLeases).unwrap()[0]
        .insert("active-address".into(), "192.0.2.1".into());
    issue(
        normalize(&evidence(rows, &[RouterRole::Ppp, RouterRole::Dhcp]).await),
        Issue::ConflictingIp,
    );
}

#[tokio::test(start_paused = true)]
async fn malformed_or_unusable_current_addresses_fail_without_customer_data_in_errors() {
    for address in ["private-invalid", "0.0.0.0", "224.0.0.1", "255.255.255.255"] {
        let mut rows = ppp();
        rows.get_mut(&Table::PppActive).unwrap()[0].insert("address".into(), address.into());
        let result = normalize(&evidence(rows, &[RouterRole::Ppp]).await);
        assert!(!result.as_ref().err().unwrap().to_string().contains(address));
        issue(result, Issue::InvalidField);
    }
}

#[tokio::test(start_paused = true)]
async fn normalized_evidence_keeps_the_reader_deadline() {
    let read = evidence(ppp(), &[RouterRole::Ppp]).await;
    tokio::time::advance(Duration::from_secs(6)).await;
    let normalized = normalize(&read).unwrap();
    tokio::time::advance(Duration::from_secs(4)).await;
    assert!(normalized.accounts().is_err());
    assert!(normalized.profiles().is_err());
    assert!(matches!(
        normalize(&read),
        Err(NormalizeError::Evidence(ReadError::Timeout(_)))
    ));
}

#[tokio::test(start_paused = true)]
async fn large_reversed_session_inventory_has_one_account_per_secret_and_stable_order() {
    let mut rows = ppp();
    let template = rows[&Table::PppSecrets][0].clone();
    rows.get_mut(&Table::PppSecrets).unwrap().clear();
    rows.get_mut(&Table::PppActive).unwrap().clear();
    for n in 1u32..=10_000 {
        let mut secret = template.clone();
        secret.insert(".id".into(), format!("*s{n:05}"));
        secret.insert("name".into(), format!("user{n}"));
        rows.get_mut(&Table::PppSecrets).unwrap().push(secret);
        rows.get_mut(&Table::PppActive).unwrap().push(row(&[
            (".id", &format!("*a{n:05}")),
            ("name", &format!("user{n}")),
            (
                "address",
                &std::net::Ipv4Addr::from(0x0a000000 + n).to_string(),
            ),
            ("service", "pppoe"),
        ]));
    }
    let first_read = evidence(rows.clone(), &[RouterRole::Ppp]).await;
    for rows in rows.values_mut() {
        rows.reverse();
    }
    let second_read = evidence(rows, &[RouterRole::Ppp]).await;
    let first = normalize(&first_read).unwrap();
    let second = normalize(&second_read).unwrap();
    assert_eq!(first.accounts().unwrap().len(), 10_000);
    assert!(first.accounts().unwrap() == second.accounts().unwrap());
}

#[tokio::test(start_paused = true)]
async fn ip_conflict_reports_the_actual_active_row_even_when_sessions_share_one_account() {
    let mut rows = ppp();
    let mut second = rows[&Table::PppActive][0].clone();
    second.insert(".id".into(), "*b".into());
    second.insert("address".into(), "192.0.2.5".into());
    let mut third = second.clone();
    third.insert(".id".into(), "*c".into());
    rows.get_mut(&Table::PppActive)
        .unwrap()
        .extend([second, third]);
    assert!(matches!(
        normalize(&evidence(rows, &[RouterRole::Ppp]).await),
        Err(NormalizeError::Row {
            table: Table::PppActive,
            index: 2,
            issue: Issue::ConflictingIp
        })
    ));
}

#[tokio::test(start_paused = true)]
async fn invalid_endpoint_macs_and_disabled_servers_do_not_become_absence() {
    for value in [
        "not-a-mac",
        "00:00:00:00:00:00",
        "ff:ff:ff:ff:ff:ff",
        "01:00:5e:00:00:01",
    ] {
        let mut rows = dhcp();
        rows.get_mut(&Table::DhcpLeases).unwrap()[0]
            .insert("active-mac-address".into(), value.into());
        // The lease stays present. Only its MAC evidence is absent.
        let normalized = normalize(&evidence(rows, &[RouterRole::Dhcp]).await).unwrap();
        let accounts = normalized.accounts().unwrap();
        assert_eq!(accounts.len(), 1, "{value}");
        assert_eq!(accounts[0].endpoints[0].mac, None, "{value}");
    }
    let mut rows = dhcp();
    rows.get_mut(&Table::DhcpServers).unwrap()[0].insert("disabled".into(), "true".into());
    issue(
        normalize(&evidence(rows, &[RouterRole::Dhcp]).await),
        Issue::DisabledServer,
    );
}

#[tokio::test(start_paused = true)]
async fn hotspot_disabled_or_duplicate_users_remain_visible_conflicts() {
    for duplicate in [false, true] {
        let mut rows = hotspot();
        let users = rows.get_mut(&Table::HotspotUsers).unwrap();
        if duplicate {
            let mut second = users[0].clone();
            second.insert(".id".into(), "*hu2".into());
            users.push(second);
        } else {
            users[0].insert("disabled".into(), "true".into());
        }
        issue(
            normalize(&evidence(rows, &[RouterRole::Hotspot]).await),
            if duplicate {
                Issue::DuplicateName
            } else {
                Issue::DisabledAccount
            },
        );
    }
}

#[tokio::test(start_paused = true)]
async fn mixed_hotspot_authentication_never_upgrades_endpoint_only_authority() {
    let mut rows = hotspot();
    let mut mac_session = rows[&Table::HotspotActive][0].clone();
    mac_session.insert(".id".into(), "*hb".into());
    mac_session.insert("address".into(), "192.0.2.8".into());
    mac_session.insert("login-by".into(), "mac".into());
    rows.get_mut(&Table::HotspotActive)
        .unwrap()
        .push(mac_session);
    for reverse in [false, true] {
        if reverse {
            rows.get_mut(&Table::HotspotActive).unwrap().reverse();
        }
        let read = evidence(rows.clone(), &[RouterRole::Hotspot]).await;
        let normalized = normalize(&read).unwrap();
        let accounts = normalized.accounts().unwrap();
        assert_eq!(accounts.len(), 1);
        assert_eq!(accounts[0].endpoints.len(), 2);
        assert!(matches!(
            accounts[0].authority,
            AuthorityEvidence::EndpointOnly { .. }
        ));
    }
}

#[tokio::test(start_paused = true)]
async fn wan_only_and_empty_complete_sources_produce_no_invented_subscribers() {
    for role in [
        RouterRole::Wan,
        RouterRole::Ppp,
        RouterRole::Dhcp,
        RouterRole::Hotspot,
    ] {
        let read = evidence(Rows::new(), &[role]).await;
        assert!(normalize(&read).unwrap().accounts().unwrap().is_empty());
    }
}

#[tokio::test(start_paused = true)]
async fn direct_dhcp_rate_input_stays_current_and_uninterpreted_without_a_default() {
    for value in [None, Some("2M/8M"), Some("not-yet-parsed")] {
        let mut rows = dhcp();
        if let Some(value) = value {
            rows.get_mut(&Table::DhcpLeases).unwrap()[0].insert("rate-limit".into(), value.into());
        }
        let read = evidence(rows, &[RouterRole::Dhcp]).await;
        assert_eq!(
            normalize(&read).unwrap().accounts().unwrap()[0]
                .direct_rate_limit
                .as_deref(),
            value
        );
    }
}

#[tokio::test(start_paused = true)]
async fn missing_or_disabled_hotspot_server_cannot_establish_current_service_membership() {
    for missing in [true, false] {
        let mut rows = hotspot();
        if missing {
            rows.get_mut(&Table::HotspotServers).unwrap().clear();
        } else {
            rows.get_mut(&Table::HotspotServers).unwrap()[0]
                .insert("disabled".into(), "true".into());
        }
        issue(
            normalize(&evidence(rows, &[RouterRole::Hotspot]).await),
            if missing {
                Issue::MissingServer
            } else {
                Issue::DisabledServer
            },
        );
    }
}

#[path = "exclusion_tests.rs"]
mod exclusions;

#[tokio::test(start_paused = true)]
async fn dhcp_lease_without_mac_keeps_the_router_read() {
    let mut rows = dhcp();
    rows.get_mut(&Table::DhcpLeases).unwrap()[0].remove("active-mac-address");
    let normalized = normalize(&evidence(rows, &[RouterRole::Dhcp]).await).unwrap();
    let accounts = normalized.accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].endpoints[0].mac, None);
    assert_eq!(accounts[0].endpoints[0].ips[0].to_string(), "192.0.2.2");
}

#[tokio::test(start_paused = true)]
async fn dhcp_lease_with_multicast_or_malformed_mac_has_no_mac() {
    for value in ["01:00:5e:00:00:01", "00:00:00:00:00:00", "not-a-mac"] {
        let mut rows = dhcp();
        rows.get_mut(&Table::DhcpLeases).unwrap()[0]
            .insert("active-mac-address".into(), value.into());
        let normalized = normalize(&evidence(rows, &[RouterRole::Dhcp]).await).unwrap();
        assert_eq!(
            normalized.accounts().unwrap()[0].endpoints[0].mac,
            None,
            "{value}"
        );
    }
}

#[tokio::test(start_paused = true)]
async fn hotspot_host_without_mac_still_fails_the_read() {
    let mut rows = hotspot();
    rows.get_mut(&Table::HotspotActive).unwrap()[0].remove("mac-address");
    issue(
        normalize(&evidence(rows, &[RouterRole::Hotspot]).await),
        Issue::MissingField,
    );
}
