use crate::protocol::Diagnostic;
use crate::rust_run_cycle_authority::run_rust_cycle_authority_payload;
use crate::rust_scheduler::scheduler_status_payload;
use crate::self_test::self_test_payload;
use bcrypt::{hash, verify, DEFAULT_COST};
use include_dir::{include_dir, Dir};
use rand::{distributions::Alphanumeric, Rng};
use rouille::{router, Request, Response};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

static WEB_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../web/dist");

#[derive(Clone, Debug)]
pub struct WebServerConfig {
    pub bind: String,
    pub config_path: String,
    pub users_path: String,
    pub install_dir: String,
}

#[derive(Clone, Debug, Serialize)]
struct SessionUser {
    username: String,
    role: String,
}

#[derive(Clone, Debug)]
struct SessionRecord {
    user: SessionUser,
    created_epoch: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct UserRecord {
    username: String,
    password_hash: String,
    role: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserStore {
    users: Vec<UserRecord>,
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug)]
struct AppState {
    config_path: String,
    users_path: String,
    install_dir: String,
    sessions: Mutex<HashMap<String, SessionRecord>>,
}

pub fn run_http_server(config: WebServerConfig) -> anyhow::Result<()> {
    let state = Arc::new(AppState {
        config_path: config.config_path.clone(),
        users_path: config.users_path.clone(),
        install_dir: config.install_dir.clone(),
        sessions: Mutex::new(HashMap::new()),
    });
    ensure_users_file(&state.users_path)?;
    eprintln!("lqosync-core Rust web server listening on {}", config.bind);
    rouille::start_server(config.bind, move |request| handle_request(request, state.clone()));
}

fn handle_request(request: &Request, state: Arc<AppState>) -> Response {
    router!(request,
        (GET) (/healthz) => {
            Response::json(&json!({"ok": true, "service": "lqosync-core"}))
        },
        (GET) (/api/healthz) => {
            Response::json(&json!({"ok": true, "service": "lqosync-core"}))
        },
        (POST) (/api/auth/login) => {
            handle_login(request, &state)
        },
        (POST) (/api/auth/logout) => {
            handle_logout(request, &state)
        },
        (GET) (/api/auth/me) => {
            match session_user(request, &state) {
                Some(user) => Response::json(&json!({"ok": true, "user": user})),
                None => unauthorized("Authentication required."),
            }
        },
        (GET) (/api/dashboard) => {
            match require_role(request, &state, "viewer") {
                Ok(user) => handle_dashboard(&state, &user),
                Err(response) => response,
            }
        },
        (GET) (/api/status) => {
            match require_role(request, &state, "viewer") {
                Ok(user) => handle_dashboard(&state, &user),
                Err(response) => response,
            }
        },
        (GET) (/api/rust/status) => {
            match require_role(request, &state, "viewer") {
                Ok(_user) => handle_rust_status(&state),
                Err(response) => response,
            }
        },
        (GET) (/api/config) => {
            match require_role(request, &state, "operator") {
                Ok(_user) => handle_config_get(&state),
                Err(response) => response,
            }
        },
        (PUT) (/api/config) => {
            match require_role(request, &state, "admin") {
                Ok(user) => handle_config_put(request, &state, &user),
                Err(response) => response,
            }
        },
        (GET) (/api/generated/csv) => {
            match require_role(request, &state, "viewer") {
                Ok(_user) => handle_generated_csv(&state),
                Err(response) => response,
            }
        },
        (GET) (/api/generated/network) => {
            match require_role(request, &state, "viewer") {
                Ok(_user) => handle_generated_network(&state),
                Err(response) => response,
            }
        },
        (GET) (/api/audit) => {
            match require_role(request, &state, "viewer") {
                Ok(_user) => handle_audit(request, &state),
                Err(response) => response,
            }
        },
        (GET) (/api/backups) => {
            match require_role(request, &state, "viewer") {
                Ok(_user) => handle_backups(&state),
                Err(response) => response,
            }
        },
        (GET) (/api/services/status) => {
            match require_role(request, &state, "viewer") {
                Ok(_user) => handle_services_status(&state),
                Err(response) => response,
            }
        },
        (POST) (/api/services/{service: String}/restart) => {
            match require_role(request, &state, "admin") {
                Ok(user) => handle_service_restart(&state, &user, &service),
                Err(response) => response,
            }
        },
        (POST) (/api/actions/dry-run) => {
            match require_role(request, &state, "operator") {
                Ok(user) => handle_dry_run(&state, &user),
                Err(response) => response,
            }
        },
        (POST) (/api/actions/run) => {
            match require_role(request, &state, "admin") {
                Ok(user) => handle_manual_run(&state, &user),
                Err(response) => response,
            }
        },
        _ => {
            serve_spa(request)
        }
    )
}

fn handle_login(request: &Request, state: &Arc<AppState>) -> Response {
    let payload: LoginRequest = match rouille::input::json_input(request) {
        Ok(value) => value,
        Err(_) => return bad_request("Invalid login payload."),
    };
    let users = match load_users(&state.users_path) {
        Ok(users) => users,
        Err(err) => return server_error("users_unavailable", &err.to_string()),
    };
    let username = payload.username.trim();
    let user = users.iter().find(|item| item.username == username);
    let Some(user) = user else {
        return unauthorized("Invalid username or password.");
    };
    let password_ok = verify(payload.password.as_str(), &user.password_hash).unwrap_or(false);
    if !password_ok {
        return unauthorized("Invalid username or password.");
    }
    let session_id = random_token();
    let session_user = SessionUser {
        username: user.username.clone(),
        role: normalize_role(&user.role),
    };
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.insert(
            session_id.clone(),
            SessionRecord {
                user: session_user.clone(),
                created_epoch: now_epoch(),
            },
        );
    }
    let config = load_config_value(&state.config_path).unwrap_or_else(|_| json!({}));
    let _ = append_audit(
        &config,
        "login_success",
        &session_user.username,
        json!({"role": session_user.role}),
    );
    Response::json(&json!({"ok": true, "user": session_user}))
        .with_additional_header(
            "Set-Cookie",
            format!("lqosync_session={session_id}; Path=/; HttpOnly; SameSite=Strict"),
        )
}

fn handle_logout(request: &Request, state: &Arc<AppState>) -> Response {
    if let Some(session_id) = cookie_value(request, "lqosync_session") {
        if let Ok(mut sessions) = state.sessions.lock() {
            sessions.remove(&session_id);
        }
    }
    Response::json(&json!({"ok": true}))
        .with_additional_header(
            "Set-Cookie",
            "lqosync_session=deleted; Path=/; Max-Age=0; HttpOnly; SameSite=Strict".to_string(),
        )
}

fn handle_dashboard(state: &Arc<AppState>, user: &SessionUser) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    let services = gather_service_status(&config);
    let recent_audit = tail_audit(&config, 25);
    let backups = list_backups(&config);
    let scheduler_status = scheduler_status_payload(&json!({"config_path": state.config_path}));
    let self_test = self_test_payload(&json!({"strict": false}));
    let runtime_state = load_runtime_state(&config);
    let routers = config
        .get("routers")
        .and_then(Value::as_array)
        .map(|items| items.len())
        .unwrap_or(0);
    let response = json!({
        "ok": true,
        "user": user,
        "summary": {
            "routers": routers,
            "scheduler_enabled": config.get("scheduler").and_then(|v| v.get("enabled")).and_then(Value::as_bool).unwrap_or(false),
            "auto_apply": config.get("app").and_then(|v| v.get("auto_apply")).and_then(Value::as_bool).unwrap_or(false),
            "full_rust_backend_authority": config.get("rust_core").and_then(|v| v.get("full_rust_backend_authority")).and_then(Value::as_bool).unwrap_or(false),
            "python_backend_service_removed": config.get("rust_core").and_then(|v| v.get("python_backend_service_removed")).and_then(Value::as_bool).unwrap_or(false),
            "backups": backups.len(),
            "audit_events": recent_audit.len(),
        },
        "services": services,
        "scheduler_status": envelope_to_json(scheduler_status),
        "self_test": envelope_to_json(self_test),
        "runtime_state": runtime_state,
        "recent_audit": recent_audit,
        "backups": backups,
        "generated_files": {
            "shaped_devices_csv": config_path(&config, &["paths", "shaped_devices_csv"], "/opt/libreqos/src/ShapedDevices.csv"),
            "network_json": config_path(&config, &["paths", "network_json"], "/opt/libreqos/src/network.json"),
        },
        "install_dir": state.install_dir,
    });
    Response::json(&response)
}

fn handle_rust_status(state: &Arc<AppState>) -> Response {
    let scheduler_status = scheduler_status_payload(&json!({"config_path": state.config_path}));
    let self_test = self_test_payload(&json!({"strict": false}));
    Response::json(&json!({
        "ok": true,
        "scheduler_status": envelope_to_json(scheduler_status),
        "self_test": envelope_to_json(self_test),
    }))
}

fn handle_config_get(state: &Arc<AppState>) -> Response {
    match load_config_value(&state.config_path) {
        Ok(value) => Response::json(&json!({"ok": true, "config": value})),
        Err(err) => server_error("config_unavailable", &err.to_string()),
    }
}

fn handle_config_put(request: &Request, state: &Arc<AppState>, user: &SessionUser) -> Response {
    let payload: Value = match rouille::input::json_input(request) {
        Ok(value) => value,
        Err(_) => return bad_request("Invalid config payload."),
    };
    let config_value = payload.get("config").cloned().unwrap_or(payload);
    if !config_value.is_object() {
        return bad_request("Config payload must be a JSON object.");
    }
    if let Err(err) = write_json_file(&state.config_path, &config_value) {
        return server_error("config_write_failed", &err.to_string());
    }
    let _ = append_audit(
        &config_value,
        "config_saved",
        &user.username,
        json!({"source": "rust_webui"}),
    );
    Response::json(&json!({"ok": true, "config": config_value}))
}

fn handle_generated_csv(state: &Arc<AppState>) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    let path = config_path(
        &config,
        &["paths", "shaped_devices_csv"],
        "/opt/libreqos/src/ShapedDevices.csv",
    );
    match fs::read_to_string(&path) {
        Ok(text) => Response::from_data("text/csv", text),
        Err(err) => server_error("csv_unavailable", &format!("read {path}: {err}")),
    }
}

fn handle_generated_network(state: &Arc<AppState>) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    let path = config_path(
        &config,
        &["paths", "network_json"],
        "/opt/libreqos/src/network.json",
    );
    match fs::read_to_string(&path) {
        Ok(text) => Response::from_data("application/json", text),
        Err(err) => server_error("network_unavailable", &format!("read {path}: {err}")),
    }
}

fn handle_audit(request: &Request, state: &Arc<AppState>) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    let limit = request
        .get_param("limit")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(100);
    Response::json(&json!({"ok": true, "events": tail_audit(&config, limit)}))
}

fn handle_backups(state: &Arc<AppState>) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    Response::json(&json!({"ok": true, "backups": list_backups(&config)}))
}

fn handle_services_status(state: &Arc<AppState>) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    Response::json(&json!({"ok": true, "services": gather_service_status(&config)}))
}

fn handle_service_restart(state: &Arc<AppState>, user: &SessionUser, service: &str) -> Response {
    let config = match load_config_value(&state.config_path) {
        Ok(value) => value,
        Err(err) => return server_error("config_unavailable", &err.to_string()),
    };
    let allowed = allowed_services(&config);
    if !allowed.iter().any(|item| item == service) {
        return forbidden("Service restart not allowed.");
    }
    let result = Command::new("systemctl").args(["restart", service]).output();
    match result {
        Ok(output) => {
            let ok = output.status.success();
            let _ = append_audit(
                &config,
                "service_restart",
                &user.username,
                json!({
                    "service": service,
                    "ok": ok,
                    "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
                }),
            );
            Response::json(&json!({
                "ok": ok,
                "service": service,
                "stdout": String::from_utf8_lossy(&output.stdout),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }))
            .with_status_code(if ok { 200 } else { 500 })
        }
        Err(err) => server_error("service_restart_failed", &err.to_string()),
    }
}

fn handle_dry_run(state: &Arc<AppState>, user: &SessionUser) -> Response {
    let payload = json!({
        "config_path": state.config_path,
        "mode": "dry_run",
        "execute": false
    });
    let envelope = envelope_to_json(run_rust_cycle_authority_payload(&payload));
    let _ = append_audit(
        &load_config_value(&state.config_path).unwrap_or_else(|_| json!({})),
        "dry_run_requested",
        &user.username,
        json!({"status": envelope.get("result").and_then(|v| v.get("status")).cloned()}),
    );
    Response::json(&envelope)
}

fn handle_manual_run(state: &Arc<AppState>, user: &SessionUser) -> Response {
    let payload = json!({
        "config_path": state.config_path,
        "mode": "manual",
        "execute": true
    });
    let envelope = envelope_to_json(run_rust_cycle_authority_payload(&payload));
    let _ = append_audit(
        &load_config_value(&state.config_path).unwrap_or_else(|_| json!({})),
        "manual_run_requested",
        &user.username,
        json!({"status": envelope.get("result").and_then(|v| v.get("status")).cloned()}),
    );
    Response::json(&envelope)
}

fn serve_spa(request: &Request) -> Response {
    let mut path = request.url().trim_start_matches('/').to_string();
    if path.is_empty() {
        path = "index.html".to_string();
    }
    if let Some(file) = WEB_DIST.get_file(&path) {
        return Response::from_data(content_type_for(&path), file.contents().to_vec());
    }
    if let Some(file) = WEB_DIST.get_file("index.html") {
        return Response::from_data("text/html; charset=utf-8", file.contents().to_vec());
    }
    Response::text("Svelte frontend build is missing.").with_status_code(503)
}

fn envelope_to_json(
    envelope: (Value, Vec<Diagnostic>, Vec<Diagnostic>),
) -> Value {
    let (result, errors, warnings) = envelope;
    json!({
        "ok": errors.is_empty(),
        "result": result,
        "errors": errors,
        "warnings": warnings,
    })
}

fn content_type_for(path: &str) -> &'static str {
    if path.ends_with(".html") {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else if path.ends_with(".json") || path.ends_with(".webmanifest") {
        "application/json; charset=utf-8"
    } else {
        "application/octet-stream"
    }
}

fn bad_request(message: &str) -> Response {
    Response::json(&json!({"ok": false, "error": message})).with_status_code(400)
}

fn unauthorized(message: &str) -> Response {
    Response::json(&json!({"ok": false, "error": message})).with_status_code(401)
}

fn forbidden(message: &str) -> Response {
    Response::json(&json!({"ok": false, "error": message})).with_status_code(403)
}

fn server_error(code: &str, message: &str) -> Response {
    Response::json(&json!({"ok": false, "code": code, "error": message})).with_status_code(500)
}

fn session_user(request: &Request, state: &Arc<AppState>) -> Option<SessionUser> {
    let session_id = cookie_value(request, "lqosync_session")?;
    let sessions = state.sessions.lock().ok()?;
    sessions.get(&session_id).map(|session| {
        let _age = now_epoch().saturating_sub(session.created_epoch);
        session.user.clone()
    })
}

fn require_role(
    request: &Request,
    state: &Arc<AppState>,
    minimum: &str,
) -> Result<SessionUser, Response> {
    let Some(user) = session_user(request, state) else {
        return Err(unauthorized("Authentication required."));
    };
    if role_rank(&user.role) < role_rank(minimum) {
        return Err(forbidden("Insufficient role for this action."));
    }
    Ok(user)
}

fn cookie_value(request: &Request, name: &str) -> Option<String> {
    let raw = request.header("Cookie")?;
    for part in raw.split(';') {
        let trimmed = part.trim();
        let mut pieces = trimmed.splitn(2, '=');
        let key = pieces.next()?.trim();
        let value = pieces.next()?.trim();
        if key == name {
            return Some(value.to_string());
        }
    }
    None
}

fn role_rank(role: &str) -> i64 {
    match normalize_role(role).as_str() {
        "viewer" => 10,
        "operator" => 20,
        "admin" => 30,
        "owner" => 40,
        _ => 0,
    }
}

fn normalize_role(role: &str) -> String {
    match role.trim().to_lowercase().as_str() {
        "owner" => "owner".to_string(),
        "admin" => "admin".to_string(),
        "operator" | "ops" => "operator".to_string(),
        _ => "viewer".to_string(),
    }
}

fn random_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

fn ensure_users_file(path: &str) -> anyhow::Result<()> {
    let users_path = Path::new(path);
    if users_path.exists() {
        return Ok(());
    }
    if let Some(parent) = users_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let password_hash = hash("adminpass", DEFAULT_COST)?;
    let store = UserStore {
        users: vec![UserRecord {
            username: "admin".to_string(),
            password_hash,
            role: "owner".to_string(),
        }],
    };
    write_json_file(path, &serde_json::to_value(store)?)?;
    Ok(())
}

fn load_users(path: &str) -> anyhow::Result<Vec<UserRecord>> {
    ensure_users_file(path)?;
    let text = fs::read_to_string(path)?;
    let mut store: UserStore = serde_json::from_str(&text)?;
    if store.users.is_empty() {
        ensure_users_file(path)?;
        let fallback = fs::read_to_string(path)?;
        store = serde_json::from_str(&fallback)?;
    }
    Ok(store.users)
}

fn load_config_value(path: &str) -> anyhow::Result<Value> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn write_json_file(path: &str, value: &Value) -> anyhow::Result<()> {
    let target = Path::new(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = format!("{path}.tmp.{}", std::process::id());
    fs::write(&tmp, serde_json::to_vec_pretty(value)?)?;
    fs::rename(&tmp, target)?;
    Ok(())
}

fn config_path(config: &Value, path: &[&str], default: &str) -> String {
    let mut current = config;
    for key in path {
        match current.get(*key) {
            Some(next) => current = next,
            None => return default.to_string(),
        }
    }
    current.as_str().unwrap_or(default).to_string()
}

fn load_runtime_state(config: &Value) -> Value {
    let path = config_path(
        config,
        &["paths", "runtime_state"],
        "/opt/LQoSync/state/runtime_state.json",
    );
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| json!({})),
        Err(_) => json!({}),
    }
}

fn append_audit(config: &Value, action: &str, actor: &str, details: Value) -> anyhow::Result<()> {
    let path = config_path(config, &["paths", "audit_log"], "/opt/LQoSync/logs/audit.jsonl");
    let target = Path::new(&path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut payload = serde_json::Map::new();
    payload.insert("ts".to_string(), json!(now_epoch()));
    payload.insert("actor".to_string(), json!(actor));
    payload.insert("action".to_string(), json!(action));
    payload.insert("details".to_string(), details);
    let line = format!("{}\n", serde_json::to_string(&Value::Object(payload))?);
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)?;
    file.write_all(line.as_bytes())?;
    Ok(())
}

fn tail_audit(config: &Value, limit: usize) -> Vec<Value> {
    let path = config_path(config, &["paths", "audit_log"], "/opt/LQoSync/logs/audit.jsonl");
    let text = fs::read_to_string(path).unwrap_or_default();
    text.lines()
        .rev()
        .take(limit.min(1000))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect()
}

fn list_backups(config: &Value) -> Vec<Value> {
    let root = PathBuf::from(config_path(
        config,
        &["paths", "backup_dir"],
        "/opt/LQoSync/backups",
    ));
    let Ok(read_dir) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut backups = read_dir
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| {
            let id = entry.file_name().to_string_lossy().to_string();
            let metadata_path = entry.path().join("metadata.json");
            let metadata = fs::read_to_string(metadata_path)
                .ok()
                .and_then(|text| serde_json::from_str::<Value>(&text).ok())
                .unwrap_or_else(|| json!({}));
            json!({
                "id": id,
                "path": entry.path(),
                "metadata": metadata,
            })
        })
        .collect::<Vec<_>>();
    backups.sort_by(|a, b| {
        b.get("id")
            .and_then(Value::as_str)
            .cmp(&a.get("id").and_then(Value::as_str))
    });
    backups
}

fn allowed_services(config: &Value) -> Vec<String> {
    let mut services = Vec::new();
    if let Some(units) = config
        .get("services")
        .and_then(|value| value.get("units"))
        .and_then(Value::as_array)
    {
        for unit in units {
            if let Some(text) = unit.as_str() {
                services.push(text.to_string());
            }
        }
    }
    if services.is_empty() {
        services = vec![
            "lqosync-core".to_string(),
            "lqosd".to_string(),
            "lqos_scheduler".to_string(),
            "lqosync".to_string(),
        ];
    } else if !services.iter().any(|item| item == "lqosync-core") {
        services.insert(0, "lqosync-core".to_string());
    }
    services
}

fn gather_service_status(config: &Value) -> Vec<Value> {
    allowed_services(config)
        .into_iter()
        .map(|service| {
            let output = Command::new("systemctl")
                .args([
                    "show",
                    &service,
                    "--property=LoadState,ActiveState,SubState,Description",
                    "--no-pager",
                ])
                .output();
            match output {
                Ok(result) => {
                    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
                    let mut props = HashMap::new();
                    for line in stdout.lines() {
                        if let Some((key, value)) = line.split_once('=') {
                            props.insert(key.to_string(), value.to_string());
                        }
                    }
                    json!({
                        "unit": service,
                        "load": props.get("LoadState").cloned().unwrap_or_else(|| "unknown".to_string()),
                        "active": props.get("ActiveState").cloned().unwrap_or_else(|| "unknown".to_string()),
                        "sub": props.get("SubState").cloned().unwrap_or_else(|| "unknown".to_string()),
                        "description": props.get("Description").cloned().unwrap_or_default(),
                    })
                }
                Err(err) => json!({
                    "unit": service,
                    "load": "unknown",
                    "active": "unknown",
                    "sub": "unknown",
                    "description": "",
                    "error": err.to_string(),
                }),
            }
        })
        .collect()
}
