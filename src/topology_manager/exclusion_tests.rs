use super::*;
use crate::config::Config;

fn config() -> Config {
    Config::from_toml_str("[[routers]]\nname='Router'\naddress='192.0.2.100'\nusername='u'\npassword_file='/not-used'\n[routers.pppoe]\nenabled=true\n[routers.hotspot]\nenabled=true\n[routers.dhcp]\nenabled=true\nrequire_bound=true\nshape_unassigned=false\n").unwrap()
}
fn all() -> Rows {
    let mut rows = ppp();
    rows.extend(dhcp());
    rows.extend(hotspot());
    rows
}
fn ips(current: &CurrentRouter) -> BTreeSet<String> {
    current
        .accounts()
        .unwrap()
        .iter()
        .flat_map(|a| &a.endpoints)
        .flat_map(|e| &e.ips)
        .map(ToString::to_string)
        .collect()
}

#[tokio::test(start_paused = true)]
async fn merged_ip_filters_keep_only_current_unexcluded_sessions() {
    let mut cfg = config();
    cfg.exclude.ips.push("192.0.2.0/31".into());
    cfg.routers[0].exclude.ips.push("192.0.2.2".into());
    let read = evidence(
        all(),
        &[RouterRole::Ppp, RouterRole::Dhcp, RouterRole::Hotspot],
    )
    .await;
    let current = normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude).unwrap();
    assert_eq!(ips(&current), BTreeSet::from(["192.0.2.3".into()]));
    assert_eq!(current.profiles().unwrap().len(), 2);
    assert_eq!(current.services().unwrap().len(), 2);
}

#[tokio::test(start_paused = true)]
async fn wildcard_profile_and_server_filters_do_not_affect_other_protocols() {
    let mut cfg = config();
    cfg.exclude.ppp_profiles.push("G*D".into());
    cfg.routers[0].exclude.dhcp_servers.push("L*N".into());
    let read = evidence(
        all(),
        &[RouterRole::Ppp, RouterRole::Dhcp, RouterRole::Hotspot],
    )
    .await;
    let current = normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude).unwrap();
    assert_eq!(ips(&current), BTreeSet::from(["192.0.2.3".into()]));
}

#[tokio::test(start_paused = true)]
async fn one_excluded_ppp_session_cannot_leave_its_comments_or_service_membership() {
    let mut cfg = config();
    cfg.exclude.ips.push("192.0.2.1".into());
    let mut rows = ppp();
    rows.get_mut(&Table::PppActive).unwrap()[0].insert("comment".into(), "100M/20M".into());
    let mut second = rows[&Table::PppActive][0].clone();
    second.insert(".id".into(), "*b".into());
    second.insert("address".into(), "2001:db8::1".into());
    second.insert("service".into(), "l2tp".into());
    second.insert("comment".into(), "50M/10M".into());
    rows.get_mut(&Table::PppActive).unwrap().push(second);
    let read = evidence(rows, &[RouterRole::Ppp]).await;
    let current = normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude).unwrap();
    assert_eq!(ips(&current), BTreeSet::from(["2001:db8::1".into()]));
    let account = &current.accounts().unwrap()[0];
    assert_eq!(account.service_refs, vec!["l2tp"]);
    assert_eq!(account.active_comments, vec![Some("50M/10M".into())]);
}

#[tokio::test(start_paused = true)]
async fn excluded_sessions_still_require_valid_account_and_endpoint_evidence() {
    let mut cfg = config();
    cfg.exclude.ppp_profiles.push("*".into());
    cfg.exclude.ips.push("192.0.2.0/24".into());
    let mut rows = ppp();
    rows.get_mut(&Table::PppSecrets).unwrap()[0].insert("disabled".into(), "true".into());
    let read = evidence(rows, &[RouterRole::Ppp]).await;
    issue(
        normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude),
        Issue::DisabledAccount,
    );
    let mut rows = dhcp();
    rows.get_mut(&Table::DhcpLeases).unwrap()[0].remove("active-mac-address");
    let read = evidence(rows, &[RouterRole::Dhcp]).await;
    issue(
        normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude),
        Issue::MissingField,
    );
}

#[tokio::test(start_paused = true)]
async fn exclusions_cannot_hide_duplicate_current_ips() {
    let mut cfg = config();
    cfg.exclude.ips.push("192.0.2.0/24".into());
    let mut rows = all();
    rows.get_mut(&Table::DhcpLeases).unwrap()[0]
        .insert("active-address".into(), "192.0.2.1".into());
    let read = evidence(
        rows,
        &[RouterRole::Ppp, RouterRole::Dhcp, RouterRole::Hotspot],
    )
    .await;
    issue(
        normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude),
        Issue::ConflictingIp,
    );
}

#[tokio::test(start_paused = true)]
async fn filters_cannot_extend_expired_source_evidence() {
    let mut cfg = config();
    cfg.exclude.ips.push("192.0.2.0/24".into());
    let read = evidence(ppp(), &[RouterRole::Ppp]).await;
    tokio::time::advance(Duration::from_secs(11)).await;
    assert!(normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude).is_err());
}

#[tokio::test(start_paused = true)]
async fn current_ipv4_filter_membership_matches_the_legacy_collector() {
    for case in 0..4 {
        let mut cfg = config();
        match case {
            0 => cfg.exclude.ips.push("192.0.2.0/31".into()),
            1 => cfg.routers[0].exclude.ppp_profiles.push("G*D".into()),
            2 => cfg.exclude.dhcp_servers.push("L*N".into()),
            _ => cfg.routers[0].exclude.ips.push("192.0.2.0/24".into()),
        }
        let rows = all();
        let mut leases = rows[&Table::DhcpLeases].clone();
        for lease in &mut leases {
            lease.insert("mac-address".into(), lease["active-mac-address"].clone());
            lease.insert("address".into(), lease["active-address"].clone());
        }
        let snap = crate::model::RouterSnapshot {
            ppp_secret: rows[&Table::PppSecrets].clone(),
            ppp_active: rows[&Table::PppActive].clone(),
            ppp_profile: rows[&Table::PppProfiles].clone(),
            dhcp_server: rows[&Table::DhcpServers].clone(),
            dhcp_lease: leases,
            hotspot_active: rows[&Table::HotspotActive].clone(),
            hotspot_user: Some(rows[&Table::HotspotUsers].clone()),
        };
        let legacy = crate::sources::collect_router(
            &cfg,
            &cfg.routers[0],
            &snap,
            &crate::speed::SpeedParser::new(&cfg.speed).unwrap(),
            &mut vec![],
        );
        let read = evidence(
            rows,
            &[RouterRole::Ppp, RouterRole::Dhcp, RouterRole::Hotspot],
        )
        .await;
        let current = normalize_configured(&read, &cfg.exclude, &cfg.routers[0].exclude).unwrap();
        assert_eq!(
            ips(&current),
            legacy.iter().map(|c| c.ipv4.clone()).collect()
        );
        assert_eq!(ips(&current).len(), if case == 3 { 0 } else { 2 });
    }
}
