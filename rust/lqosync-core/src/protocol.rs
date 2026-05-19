use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::time::Instant;

pub const PROTOCOL_VERSION: &str = "1";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoreRequest {
    #[serde(default = "default_version")]
    pub version: String,
    pub op: String,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safe_for_cleanup: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CoreResponse {
    pub version: String,
    pub op: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub ok: bool,
    #[serde(default)]
    pub result: Value,
    #[serde(default)]
    pub errors: Vec<Diagnostic>,
    #[serde(default)]
    pub warnings: Vec<Diagnostic>,
    #[serde(default)]
    pub meta: Map<String, Value>,
}

fn default_version() -> String {
    PROTOCOL_VERSION.to_string()
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, path: impl Into<Option<String>>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Error,
            path: path.into(),
            message: message.into(),
            value: None,
            safe_for_cleanup: None,
        }
    }

    pub fn warning(code: impl Into<String>, path: impl Into<Option<String>>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            severity: Severity::Warning,
            path: path.into(),
            message: message.into(),
            value: None,
            safe_for_cleanup: None,
        }
    }

    pub fn with_value(mut self, value: Value) -> Self {
        self.value = Some(value);
        self
    }

    pub fn with_safe_for_cleanup(mut self, safe: bool) -> Self {
        self.safe_for_cleanup = Some(safe);
        self
    }
}

impl CoreResponse {
    pub fn success(req: &CoreRequest, result: Value, started: Instant) -> Self {
        let mut meta = Map::new();
        meta.insert("engine".to_string(), json!("lqosync-core"));
        meta.insert("duration_ms".to_string(), json!(duration_ms(started)));
        Self {
            version: PROTOCOL_VERSION.to_string(),
            op: req.op.clone(),
            request_id: req.request_id.clone(),
            ok: true,
            result,
            errors: vec![],
            warnings: vec![],
            meta,
        }
    }

    pub fn validation(req: &CoreRequest, result: Value, errors: Vec<Diagnostic>, warnings: Vec<Diagnostic>, started: Instant) -> Self {
        let mut meta = Map::new();
        meta.insert("engine".to_string(), json!("lqosync-core"));
        meta.insert("duration_ms".to_string(), json!(duration_ms(started)));
        Self {
            version: PROTOCOL_VERSION.to_string(),
            op: req.op.clone(),
            request_id: req.request_id.clone(),
            ok: errors.is_empty(),
            result,
            errors,
            warnings,
            meta,
        }
    }

    pub fn failure(op: impl Into<String>, request_id: Option<String>, code: &str, message: impl Into<String>, started: Instant) -> Self {
        let mut meta = Map::new();
        meta.insert("engine".to_string(), json!("lqosync-core"));
        meta.insert("duration_ms".to_string(), json!(duration_ms(started)));
        Self {
            version: PROTOCOL_VERSION.to_string(),
            op: op.into(),
            request_id,
            ok: false,
            result: json!({}),
            errors: vec![Diagnostic::error(code, None, message.into())],
            warnings: vec![],
            meta,
        }
    }
}

fn duration_ms(started: Instant) -> f64 {
    let elapsed = started.elapsed();
    (elapsed.as_secs_f64() * 1000.0 * 1000.0).round() / 1000.0
}
