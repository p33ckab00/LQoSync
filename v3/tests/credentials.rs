use lqosync_v3::credentials;
mod common;
#[tokio::test]
async fn local_toml_passwords_are_selected_by_router_and_errors_never_quote_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let router = common::fixture().0.routers.remove(0);
    let path = dir.path().join("credentials.toml");
    std::fs::write(&path, "[routers.r1]\npassword='fake-private-value'\n").unwrap();
    assert_eq!(
        credentials::password(dir.path(), &router).await.unwrap(),
        "fake-private-value"
    );
    std::fs::write(&path, "[routers.r1]\npassword='fake-private-value\n").unwrap();
    let err = credentials::password(dir.path(), &router)
        .await
        .unwrap_err()
        .to_string();
    assert!(!err.contains("fake-private-value"));
    assert!(err.contains("TOML"));
    std::fs::write(&path, "[routers.r1]\npassword=''\n").unwrap();
    assert!(credentials::password(dir.path(), &router).await.is_err());
    std::fs::write(&path, "[routers.other]\npassword='not-r1'\n").unwrap();
    assert!(credentials::password(dir.path(), &router).await.is_err());
}
#[tokio::test]
async fn environment_credentials_remain_compatible_when_no_local_entry_exists() {
    let dir = tempfile::tempdir().unwrap();
    let mut router = common::fixture().0.routers.remove(0);
    router.password_env = "PATH".into();
    assert_eq!(
        credentials::password(dir.path(), &router).await.unwrap(),
        std::env::var("PATH").unwrap()
    );
    std::fs::write(
        dir.path().join("credentials.toml"),
        "[routers.other]\npassword='not-used'\n",
    )
    .unwrap();
    assert_eq!(
        credentials::password(dir.path(), &router).await.unwrap(),
        std::env::var("PATH").unwrap()
    );
}
