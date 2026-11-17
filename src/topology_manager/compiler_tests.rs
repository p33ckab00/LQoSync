use super::super::publication::{prepare, tests::input};
use super::*;

pub(crate) fn fixture() -> (
    tempfile::TempDir,
    crate::config::Libreqos,
    ControlledCompiler,
) {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("native");
    let state = root.path().join("native-state");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&state).unwrap();
    std::fs::write(source.join("liblqos_python.so"), b"binding-v1").unwrap();
    let conf = root.path().join("native.conf");
    std::fs::write(&conf,format!("lqos_directory={:?}\nstate_directory={:?}\n[topology]\nsource=\"shaped_devices\"\ncompile_mode=\"full\"\n",source,state)).unwrap();
    let cfg = crate::config::Libreqos {
        directory: source.to_string_lossy().into_owned(),
        state_directory: state.to_string_lossy().into_owned(),
        v3_config: conf.to_string_lossy().into_owned(),
        ..Default::default()
    };
    let compiler = ControlledCompiler::open(&cfg, &root.path().join("scratch")).unwrap();
    (root, cfg, compiler)
}
pub(crate) fn script(pair: &PreparedPair) -> String {
    let (canonical, shaping) = super::super::admission::tests::artifacts(pair);
    format!(
        "import os\ns=os.environ['COMPILER_STATE']\nos.makedirs(s+'/topology')\nos.makedirs(s+'/shaping')\nopen(s+'/topology/topology_canonical_state.json','w').write({})\nopen(s+'/shaping/topology_compiled_shaping.json','w').write({})\n",
        serde_json::to_string(&canonical.to_string()).unwrap(),
        serde_json::to_string(&shaping.to_string()).unwrap()
    )
}
pub(crate) async fn compiled(
    compiler: &mut ControlledCompiler,
    pair: PreparedPair,
) -> CompiledPair {
    let code = script(&pair);
    compiler.compile_code(pair, &code).await.unwrap()
}
#[test]
fn compiler_snapshot_preserves_exact_bytes_without_a_text_newline() {
    let (_root, cfg, compiler) = fixture();
    assert_eq!(
        std::fs::read(compiler.cache.0.join("liblqos_python.so")).unwrap(),
        std::fs::read(Path::new(&cfg.directory).join("liblqos_python.so")).unwrap()
    );
}
#[tokio::test]
async fn controlled_child_artifacts_admit_only_the_exact_fresh_pair() {
    let (_root, _cfg, mut compiler) = fixture();
    let pair = prepare(input(1)).unwrap();
    let bound = compiled(&mut compiler, pair).await;
    bound.binding().check_current().unwrap();
    let pair = prepare(input(1)).unwrap();
    let code = script(&prepare(input(0)).unwrap());
    assert!(compiler.compile_code(pair, &code).await.is_err());
}
#[tokio::test]
async fn drift_and_missing_outputs_block_without_a_native_apply() {
    let (_root, cfg, mut compiler) = fixture();
    assert!(
        compiler
            .compile_code(prepare(input(1)).unwrap(), "pass")
            .await
            .is_err()
    );
    std::fs::write(
        Path::new(&cfg.directory).join("liblqos_python.so"),
        b"binding-v2",
    )
    .unwrap();
    assert!(
        compiler
            .compile_code(prepare(input(1)).unwrap(), "pass")
            .await
            .is_err()
    );
}
#[tokio::test]
async fn deadline_and_mid_compile_config_drift_reject_otherwise_valid_artifacts() {
    let (_root, cfg, mut compiler) = fixture();
    let mut observed = input(1);
    observed.expires = tokio::time::Instant::now() + Duration::from_millis(60);
    let pair = prepare(observed).unwrap();
    let code = format!("import time\ntime.sleep(0.2)\n{}", script(&pair));
    assert!(compiler.compile_code(pair, &code).await.is_err());
    let pair = prepare(input(1)).unwrap();
    let code = format!("import time\ntime.sleep(0.2)\n{}", script(&pair));
    let path = cfg.v3_config;
    let change = async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let mut text = std::fs::read_to_string(&path).unwrap();
        text.push_str("\n# changed\n");
        std::fs::write(path, text).unwrap();
    };
    let (result, ()) = tokio::join!(compiler.compile_code(pair, &code), change);
    assert!(result.is_err());
}

#[tokio::test]
#[ignore = "requires explicit installed-binding scratch compilation"]
async fn installed_binding_preserves_the_pair_inside_the_kernel_boundary() {
    let source =
        std::env::var("LQOSYNC3_RUNTIME_BINDING").expect("set LQOSYNC3_RUNTIME_BINDING explicitly");
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("lqos.conf");
    std::fs::write(
        &config,
        std::fs::read(Path::new(&source).join("lqos.example")).unwrap(),
    )
    .unwrap();
    let text = std::fs::read_to_string(&config).unwrap();
    let table: toml::Table = toml::from_str(&text).unwrap();
    let cfg = crate::config::Libreqos {
        directory: source,
        state_directory: table["state_directory"].as_str().unwrap().into(),
        v3_config: config.to_string_lossy().into_owned(),
        ..Default::default()
    };
    let mut compiler = ControlledCompiler::open(&cfg, &root.path().join("scratch")).unwrap();
    let mut observed = input(1);
    for node in &mut observed
        .reconciliation
        .registry
        .catalog
        .as_mut()
        .unwrap()
        .nodes
    {
        if node.source_id.as_deref() == Some("profile-source") {
            node.minimum.down = Some(2_000_000);
        }
    }
    let compiled = compiler.compile(prepare(observed).unwrap()).await.unwrap();
    compiled.check().unwrap();
    println!(
        "Installed compiler preserves the pair; sockets and outside content writes are blocked."
    );
}

pub(crate) fn binding(compiler: &ControlledCompiler) -> CompilerBinding {
    compiler.binding.clone()
}

#[tokio::test]
async fn workspace_paths_do_not_change_effective_identity_and_expired_permits_fail() {
    let (_root, _cfg, mut compiler) = fixture();
    let first = compiled(&mut compiler, prepare(input(1)).unwrap()).await;
    let second = compiled(&mut compiler, prepare(input(1)).unwrap()).await;
    assert_eq!(first.binding(), second.binding());
    tokio::time::pause();
    tokio::time::advance(Duration::from_secs(11)).await;
    assert!(second.check().is_err());
}
#[test]
fn regular_file_bounds_and_active_state_directory_are_enforced() {
    let (root, mut cfg, compiler) = fixture();
    let path = root.path().join("oversize");
    std::fs::write(&path, b"123").unwrap();
    assert!(read_regular(&path, 2, true).is_err());
    let link = root.path().join("link");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    assert!(read_regular(&link, 100, true).is_err());
    let other = root.path().join("other-state");
    std::fs::create_dir(&other).unwrap();
    cfg.state_directory = other.to_string_lossy().into_owned();
    assert!(compiler.check_config(&cfg).is_err());
}
#[tokio::test]
async fn a_child_cannot_change_inputs_or_native_topology_configuration() {
    for change in [
        "open('src/network.json','w').write('{}')",
        "p=os.environ['LQOS_CONFIG'];t=open(p).read();open(p,'w').write(t.replace('full','ap_site'))",
    ] {
        let (_root, _cfg, mut compiler) = fixture();
        let pair = prepare(input(1)).unwrap();
        let code = format!("{}\n{}", script(&pair), change);
        assert!(compiler.compile_code(pair, &code).await.is_err());
    }
}

#[test]
fn native_configuration_errors_do_not_expose_raw_config_text() {
    let (root, cfg, _compiler) = fixture();
    std::fs::write(&cfg.v3_config, b"secret=\"private-credential\"\ninvalid=[").unwrap();
    let error = ControlledCompiler::open(&cfg, &root.path().join("other-scratch"))
        .err()
        .unwrap();
    assert!(!format!("{error:#}").contains("private-credential"));
}
