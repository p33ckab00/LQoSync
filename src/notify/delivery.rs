//! API receipts and bounded HTML parts for durable delivery.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiReceipt {
    pub message_id: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliveryError {
    pub reason: String,
    pub retry_after: Option<u64>,
    pub permanent: bool,
}
impl DeliveryError {
    pub fn transient(reason: &str) -> Self {
        Self {
            reason: reason.into(),
            retry_after: None,
            permanent: false,
        }
    }
}
impl std::fmt::Display for DeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.reason)
    }
}
pub async fn post_receipt<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    host: &str,
    path: &str,
    body: &str,
) -> Result<ApiReceipt, DeliveryError> {
    let (code, value) = super::response(stream, host, path, body)
        .await
        .map_err(|_| DeliveryError::transient("telegram_response_unavailable"))?;
    if (200..300).contains(&code) && value["ok"] == true {
        if let Some(id) = value["result"]["message_id"].as_i64().filter(|id| *id > 0) {
            return Ok(ApiReceipt { message_id: id });
        }
        return Err(DeliveryError::transient("telegram_receipt_missing"));
    }
    let api_code = value["error_code"].as_u64().unwrap_or(u64::from(code));
    Err(DeliveryError {
        reason: format!("telegram_api_{api_code}"),
        retry_after: value["parameters"]["retry_after"].as_u64(),
        permanent: (400..500).contains(&api_code) && api_code != 429,
    })
}
/// Split only text atoms. Close and reopen supported tags; never split an entity or Unicode scalar.
/// The raw UTF-16 length is conservative because Telegram counts parsed text characters.
pub fn html_parts(text: &str, max: usize) -> Result<Vec<String>, String> {
    let max = max.clamp(128, 4096);
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut current_len = 0usize;
    let mut tags: Vec<&str> = Vec::new();
    let mut offset = 0;
    while offset < text.len() {
        let tail = &text[offset..];
        let atom = if tail.starts_with('<') {
            let end = tail.find('>').ok_or("unterminated HTML tag")?;
            &tail[..=end]
        } else if tail.starts_with('&') {
            let end = tail
                .find(';')
                .filter(|n| *n <= 12)
                .ok_or("unterminated HTML entity")?;
            &tail[..=end]
        } else {
            &tail[..tail.chars().next().unwrap().len_utf8()]
        };
        let mut next = tags.clone();
        if atom.starts_with('<') {
            match atom {
                "<b>" => next.push("b"),
                "<code>" => next.push("code"),
                "</b>" => {
                    if next.pop() != Some("b") {
                        return Err("unbalanced HTML".into());
                    }
                }
                "</code>" => {
                    if next.pop() != Some("code") {
                        return Err("unbalanced HTML".into());
                    }
                }
                _ => return Err("unsupported HTML tag".into()),
            }
            if next.len() > 8 {
                return Err("HTML nesting exceeds bound".into());
            }
        }
        let closers = |t: &[&str]| {
            t.iter()
                .rev()
                .map(|t| format!("</{t}>"))
                .collect::<String>()
        };
        let suffix_len: usize = next.iter().map(|t| t.len() + 3).sum();
        let atom_len = atom.encode_utf16().count();
        if current_len + atom_len + suffix_len > max {
            current.push_str(&closers(&tags));
            parts.push(std::mem::take(&mut current));
            for tag in &tags {
                current.push_str(&format!("<{tag}>"));
            }
            current_len = current.len();
        }
        current.push_str(atom);
        current_len += atom_len;
        tags = next;
        offset += atom.len();
    }
    if !tags.is_empty() {
        return Err("unclosed HTML tag".into());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}
#[cfg(test)]
mod tests {
    use super::*;
    async fn answer(body: &str, code: u16) -> Result<ApiReceipt, DeliveryError> {
        let (client, mut server) = tokio::io::duplex(8192);
        let response = format!(
            "HTTP/1.1 {code} Test\r\nContent-Length: {}\r\n\r\n{body}",
            body.len()
        );
        tokio::spawn(async move {
            let mut b = [0; 4096];
            let _ = server.read(&mut b).await;
            server.write_all(response.as_bytes()).await.unwrap();
        });
        post_receipt(client, "host", "/botSECRET/sendMessage", "{}").await
    }
    #[tokio::test]
    async fn api_success_requires_message_receipt() {
        assert_eq!(
            answer(r#"{"ok":true,"result":{"message_id":17}}"#, 200)
                .await
                .unwrap()
                .message_id,
            17
        );
        assert!(answer(r#"{"ok":true}"#, 200).await.is_err());
        assert!(
            answer(r#"{"ok":false,"error_code":400}"#, 200)
                .await
                .unwrap_err()
                .permanent
        );
        let e=answer(r#"{"ok":false,"error_code":429,"parameters":{"retry_after":42},"description":"SECRET"}"#,429).await.unwrap_err();
        assert_eq!(e.retry_after, Some(42));
        assert!(!e.permanent);
        assert!(!e.reason.contains("SECRET"));
        assert!(
            answer(r#"{"ok":true,"result":{"message_id":1}} trailing"#, 200)
                .await
                .is_err()
        );
    }
    #[test]
    fn unicode_entities_and_tags_survive_parts() {
        let text = format!(
            "<b>Title</b>\n<code>{}</code>",
            "😀 &amp; &lt;client&gt; ".repeat(100)
        );
        let parts = html_parts(&text, 128).unwrap();
        assert!(parts.len() > 2);
        for part in parts {
            assert!(part.encode_utf16().count() <= 128);
            assert_eq!(
                part.matches("<code>").count(),
                part.matches("</code>").count()
            );
            assert!(!part.ends_with('&'));
        }
        assert!(html_parts("<b>bad</code>", 128).is_err());
    }
}

#[cfg(test)]
mod framing_tests {
    use super::*;
    #[tokio::test]
    async fn a_positive_json_prefix_is_not_a_complete_api_receipt() {
        for raw in [
            "HTTP/1.1 200 OK\r\nContent-Length: 500\r\n\r\n{\"ok\":true,\"result\":{\"message_id\":1}}",
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n26\r\n{\"ok\":true,\"result\":{\"message_id\":1}}\r\n5\r\n0\r\n\r\n",
        ] {
            let (client, mut server) = tokio::io::duplex(8192);
            let raw = raw.to_string();
            tokio::spawn(async move {
                let mut b = [0; 4096];
                let _ = server.read(&mut b).await;
                server.write_all(raw.as_bytes()).await.unwrap();
            });
            assert!(
                post_receipt(client, "h", "/botSECRET/x", "{}")
                    .await
                    .is_err()
            );
        }
    }
    #[test]
    fn long_subscriber_html_has_bounded_parts() {
        let text = format!(
            "<b>Subscribers</b>\n<code>{}</code>",
            "😀 &amp; client ".repeat(6000)
        );
        let start = std::time::Instant::now();
        let parts = html_parts(&text, 4096).unwrap();
        assert!(parts.iter().all(|p| p.encode_utf16().count() <= 4096));
        eprintln!(
            "HTML {} bytes / {} parts / {:?} debug",
            text.len(),
            parts.len(),
            start.elapsed()
        );
    }
}
