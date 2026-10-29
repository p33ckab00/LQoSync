use lqosync_v3::libreqos::*;

#[test]
fn selects_supported_legacy_and_v3_families() {
    for v in ["2.0.1", "2.2.202608181454", "1:2.2.202608181454-1"] {
        assert_eq!(select(v).unwrap(), Adapter::Legacy);
    }
    for v in ["3.0.7.202610271607", "3.0.0-beta12", "3.1.0"] {
        assert_eq!(select(v).unwrap(), Adapter::V3);
    }
}
#[test]
fn rejects_unknown_versions_instead_of_guessing() {
    for v in [
        "",
        "v3",
        "garbage",
        "2",
        "0.1.0",
        "2.3.0",
        "4.0.0",
        "1.5.0",
        "3.0.bad",
        "x:3.0.0",
        "1:2:3.0.0",
        "3.0.",
    ] {
        assert!(select(v).is_err(), "{v}");
    }
}
#[test]
fn only_main_installed_package_is_authoritative() {
    let text = "libreqos-preflight\t3.0.7.202610271607\tinstalled\nlibreqos\t2.2.202608181454\tinstalled\n";
    assert_eq!(installed_version(text).unwrap(), "2.2.202608181454");
    for text in [
        "libreqos-preflight\t3.0.0\tinstalled\n",
        "libreqos\t3.0.0\tunpacked\n",
        "libreqos\t3.0.0\tconfig-files\n",
    ] {
        assert!(installed_version(text).is_err());
    }
}
