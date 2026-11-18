//! Wire format: one JSON object per line. Requests and responses carry `"v": 1`.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub const VERSION: u32 = 1;
/// Maximum request line, without the newline.
pub const MAX_REQUEST: usize = 1 << 20;
/// Maximum response line, without the newline.
pub const MAX_RESPONSE: usize = 16 << 20;

fn empty_args() -> Value {
    json!({})
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub v: u32,
    pub id: String,
    pub op: String,
    #[serde(default = "empty_args")]
    pub args: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    UnsupportedVersion,
    UnknownOp,
    Unauthorized,
    Conflict,
    Invalid,
    Busy,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: String,
}
impl ApiError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}
impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}
impl std::error::Error for ApiError {}

fn line(value: &Value) -> String {
    let mut text = serde_json::to_string(value).expect("JSON values serialize");
    text.push('\n');
    text
}

pub fn response_err(id: &str, error: &ApiError) -> String {
    line(&json!({"v": VERSION, "id": id, "ok": false, "error": error}))
}

/// A response above `MAX_RESPONSE` becomes an `internal` error, never a truncated line.
pub fn response_ok(id: &str, result: Value) -> String {
    let text = line(&json!({"v": VERSION, "id": id, "ok": true, "result": result}));
    if text.len() - 1 > MAX_RESPONSE {
        return response_err(
            id,
            &ApiError::new(ErrorCode::Internal, "response exceeds 16 MiB"),
        );
    }
    text
}

/// Decode one request line. The error carries the request ID when the line names one.
pub fn parse_line(bytes: &[u8]) -> Result<Request, (String, ApiError)> {
    let value: Value = serde_json::from_slice(bytes).map_err(|e| {
        (
            String::new(),
            ApiError::new(ErrorCode::BadRequest, format!("invalid JSON: {e}")),
        )
    })?;
    let id = value
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if value.get("v").and_then(Value::as_u64) != Some(u64::from(VERSION)) {
        return Err((
            id,
            ApiError::new(
                ErrorCode::UnsupportedVersion,
                "only protocol version 1 is supported",
            ),
        ));
    }
    serde_json::from_value(value).map_err(|e| {
        (
            id,
            ApiError::new(ErrorCode::BadRequest, format!("invalid request: {e}")),
        )
    })
}

/// Read one line without its newline. A line above `max` is an error before it is fully read.
pub async fn read_line<R: AsyncBufRead + Unpin>(
    reader: &mut R,
    max: usize,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut out = Vec::new();
    loop {
        let buffer = reader.fill_buf().await?;
        if buffer.is_empty() {
            return Ok((!out.is_empty()).then_some(out));
        }
        let (take, done) = match buffer.iter().position(|&b| b == b'\n') {
            Some(i) => (i, true),
            None => (buffer.len(), false),
        };
        if out.len() + take > max {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "request line exceeds the size limit",
            ));
        }
        out.extend_from_slice(&buffer[..take]);
        reader.consume(take + usize::from(done));
        if done {
            return Ok(Some(out));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_valid_request_parses_with_default_args() {
        let r = parse_line(br#"{"v":1,"id":"a","op":"status"}"#).unwrap();
        assert_eq!((r.v, r.id.as_str(), r.op.as_str()), (1, "a", "status"));
        assert_eq!(r.args, json!({}));
    }

    #[test]
    fn bad_json_is_bad_request() {
        let (id, e) = parse_line(b"{not json").unwrap_err();
        assert_eq!((id.as_str(), e.code), ("", ErrorCode::BadRequest));
    }

    #[test]
    fn version_2_is_unsupported_version() {
        let (id, e) = parse_line(br#"{"v":2,"id":"x","op":"status"}"#).unwrap_err();
        assert_eq!((id.as_str(), e.code), ("x", ErrorCode::UnsupportedVersion));
    }

    #[test]
    fn unknown_fields_are_bad_request() {
        let (id, e) = parse_line(br#"{"v":1,"id":"y","op":"status","extra":1}"#).unwrap_err();
        assert_eq!((id.as_str(), e.code), ("y", ErrorCode::BadRequest));
    }

    #[tokio::test]
    async fn an_oversize_line_is_an_error() {
        let mut big = vec![b'a'; 2048];
        big.push(b'\n');
        let mut r = tokio::io::BufReader::new(&big[..]);
        assert!(read_line(&mut r, 1024).await.is_err());
        let mut ok = tokio::io::BufReader::new(&b"one\ntwo"[..]);
        assert_eq!(
            read_line(&mut ok, 1024).await.unwrap(),
            Some(b"one".to_vec())
        );
        assert_eq!(
            read_line(&mut ok, 1024).await.unwrap(),
            Some(b"two".to_vec())
        );
        assert_eq!(read_line(&mut ok, 1024).await.unwrap(), None);
    }

    #[test]
    fn an_oversize_response_becomes_an_internal_error() {
        let line = response_ok("z", json!("x".repeat(MAX_RESPONSE + 1)));
        let v: serde_json::Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(v["ok"], json!(false));
        assert_eq!(v["error"]["code"], json!("internal"));
        assert!(line.len() < 1024);
    }

    #[test]
    fn responses_end_with_one_newline() {
        let ok = response_ok("a", json!({"n": 1}));
        let err = response_err("b", &ApiError::new(ErrorCode::Conflict, "stale"));
        for line in [ok, err] {
            assert!(line.ends_with('\n') && !line[..line.len() - 1].contains('\n'));
        }
    }
}
