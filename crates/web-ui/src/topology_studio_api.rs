//! Typed Studio transport. Old topology endpoints keep their existing error handling.
use gloo_net::http::{Request, Response};
use leptos::prelude::*;
use web_contract::{Principal, source_studio::*, topology::Operation};
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApiError {
    pub status: Option<u16>,
    pub code: String,
    pub message: String,
    pub uncertain: bool,
}
impl ApiError {
    pub fn local(message: impl Into<String>) -> Self {
        Self {
            status: None,
            code: "invalid".into(),
            message: message.into(),
            uncertain: false,
        }
    }
}
pub(crate) fn encoded(s: &str) -> String {
    js_sys::encode_uri_component(s).into()
}
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum FailureBody {
    Studio(StudioError),
    Router(web_contract::routers::RouterEditOutcome),
}
struct Client {
    session: RwSignal<Option<Principal>>,
    actor: String,
    csrf: String,
}
impl Client {
    fn new(session: RwSignal<Option<Principal>>) -> Result<Self, ApiError> {
        let p = session
            .get_untracked()
            .ok_or_else(|| ApiError::local("Sign in before a Studio request."))?;
        Ok(Self {
            session,
            actor: p.username,
            csrf: p.csrf,
        })
    }
    fn current(&self) -> bool {
        self.session
            .try_get_untracked()
            .flatten()
            .is_some_and(|p| p.username == self.actor && p.csrf == self.csrf)
    }
    async fn response<B: serde::Serialize>(
        &self,
        url: &str,
        method: &str,
        body: Option<&B>,
    ) -> Result<Response, ApiError> {
        if !self.current() {
            return Err(ApiError::local(
                "The session changed. The old read result is ignored.",
            ));
        }
        let (signal, _timer) = crate::app::deadline();
        let builder = match method {
            "patch" => Request::patch(url).header("x-csrf-token", &self.csrf),
            "post" => Request::post(url).header("x-csrf-token", &self.csrf),
            _ => Request::get(url),
        }
        .abort_signal(Some(&signal));
        let request = match body {
            Some(body) => builder.json(body),
            None => builder.build(),
        }
        .map_err(|_| ApiError::local("The Studio request could not be prepared."))?;
        let result = request.send().await;
        if !self.current() {
            return Err(ApiError::local(
                "The session changed. The old read result is ignored.",
            ));
        }
        let response = result.map_err(|_| ApiError {
            status: None,
            code: "network".into(),
            message: "The read answer is unknown. Recover this request before a retry.".into(),
            uncertain: true,
        })?;
        if response.ok() {
            return Ok(response);
        }
        let status = response.status();
        if status == 401 {
            self.session.set(None);
        }
        let body = response.json::<FailureBody>().await.ok().map(|e| match e {
            FailureBody::Studio(e) => e,
            FailureBody::Router(e) => StudioError {
                code: e.code,
                message: e.message,
                field: None,
            },
        });
        let message = body.as_ref().map(|e| e.message.clone()).unwrap_or_else(|| {
            match status {
                401 => "Your session ended. Sign in to recover your non-secret draft.",
                403 => "The service refused this request. Check access before a retry.",
                404 => "This Studio resource is unavailable.",
                409 => "The configuration changed. Reload its cached evidence; your draft stays.",
                410 => "The preview expired. Evaluate the draft again.",
                413 => "The Studio request is too large.",
                429 => "The service is busy. Recover or retry this same request explicitly.",
                _ => "The read answer is unknown. Recover this request before a retry.",
            }
            .into()
        });
        Err(ApiError {
            status: Some(status),
            code: body.map(|e| e.code).unwrap_or_else(|| status.to_string()),
            message,
            uncertain: status >= 500,
        })
    }
    async fn get<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, ApiError> {
        self.response::<StudioJobRequest>(url, "get", None)
            .await?
            .json()
            .await
            .map_err(|_| ApiError::local("The Studio response is not valid."))
    }
    async fn settle(
        &self,
        mut op: Operation,
        key: &str,
        seconds: u64,
    ) -> Result<Operation, ApiError> {
        let end = js_sys::Date::now() + seconds.min(600) as f64 * 1000.;
        loop {
            if op.key != key {
                return Err(ApiError::local(
                    "The operation does not match this read request.",
                ));
            }
            match op.state.as_str() {
                "done" | "saved" | "failed" | "not_committed" | "outcome_unknown" | "not_found" => {
                    return Ok(op);
                }
                "queued" | "running" => {}
                _ => {
                    return Err(ApiError::local(
                        "The read job failed or stopped. Check its evidence and retry explicitly.",
                    ));
                }
            }
            if js_sys::Date::now() >= end {
                return Err(ApiError {
                    status: None,
                    code: "deadline".into(),
                    message: "The read is still pending. Recover the same request explicitly."
                        .into(),
                    uncertain: true,
                });
            }
            gloo_timers::future::TimeoutFuture::new(500).await;
            op = self
                .get(&format!("/api/v1/topology/operations/{}", encoded(key)))
                .await?;
        }
    }
}
pub(crate) async fn get<T: serde::de::DeserializeOwned>(
    session: RwSignal<Option<Principal>>,
    url: &str,
) -> Result<T, ApiError> {
    Client::new(session)?.get(url).await
}
pub(crate) async fn job(
    session: RwSignal<Option<Principal>>,
    request: &StudioJobRequest,
    seconds: u64,
    resume: bool,
) -> Result<Operation, ApiError> {
    let client = Client::new(session)?;
    let operation_url = format!("/api/v1/topology/operations/{}", encoded(&request.key));
    if resume {
        match client.get::<Operation>(&operation_url).await {
            Ok(op) if op.state == "not_found" => {}
            Ok(op) => {
                return read_result(
                    client
                        .settle(op, &request.key, seconds.min(JOB_DEADLINE_SECS))
                        .await?,
                );
            }
            Err(e) if e.status == Some(404) => {}
            Err(e) => return Err(e),
        }
    }
    let encoded_size = serde_json::to_vec(request)
        .map_err(|_| ApiError::local("The Studio request could not be prepared."))?
        .len();
    if encoded_size > MAX_JOB_BODY {
        return Err(ApiError::local(
            "The Studio request is larger than 32 KiB. Keep the draft and reduce it.",
        ));
    }
    let result = match client
        .response("/api/v1/topology/source-jobs", "post", Some(request))
        .await
    {
        Ok(r) => r.json::<Operation>().await.map_err(|_| ApiError {
            status: None,
            code: "invalid_reply".into(),
            message: "The read reply is not valid. Recover the same request.".into(),
            uncertain: true,
        }),
        Err(e) => Err(e),
    };
    let op = match result {
        Ok(op) => op,
        Err(e) if e.uncertain => match client.get::<Operation>(&operation_url).await {
            Ok(op) => op,
            Err(_) => return Err(e),
        },
        Err(e) => return Err(e),
    };
    read_result(
        client
            .settle(op, &request.key, seconds.min(JOB_DEADLINE_SECS))
            .await?,
    )
}

fn read_result(op: Operation) -> Result<Operation, ApiError> {
    if op.state == "done" {
        Ok(op)
    } else {
        Err(ApiError::local(
            op.error
                .or_else(|| op.receipt.and_then(|r| r.error))
                .unwrap_or("The read job failed. Check its evidence and retry explicitly.".into()),
        ))
    }
}
pub(crate) async fn edit(
    session: RwSignal<Option<Principal>>,
    request: &web_contract::topology::EditRequest,
    resume: bool,
    allow_resubmit: bool,
) -> Result<Operation, ApiError> {
    let client = Client::new(session)?;
    let url = format!("/api/v1/topology/operations/{}", encoded(&request.key));
    if resume {
        match client.get::<Operation>(&url).await {
            Ok(op) if op.state == "not_found" && allow_resubmit => {}
            Ok(op) if op.state == "not_found" => {
                return Err(ApiError {status:Some(404),code:"absent_changed".into(),message:"The old operation is absent and the draft changed. No old payload was resubmitted.".into(),uncertain:false});
            }
            Ok(op) => return client.settle(op, &request.key, 600).await,
            Err(e) if e.status == Some(404) && allow_resubmit => {}
            Err(e) if e.status == Some(404) => {
                return Err(ApiError {status:Some(404),code:"absent_changed".into(),message:"The old operation is absent and the draft changed. No old payload was resubmitted.".into(),uncertain:false});
            }
            Err(e) => return Err(e),
        }
    }
    let result = match client
        .response("/api/v1/topology/edits", "post", Some(request))
        .await
    {
        Ok(r) => r.json::<Operation>().await.map_err(|_| ApiError {
            status: None,
            code: "invalid_reply".into(),
            message:
                "The save answer is unknown. Recover the original request before another save."
                    .into(),
            uncertain: true,
        }),
        Err(e) => Err(e),
    };
    let op = match result {
        Ok(op) => op,
        Err(e) if e.uncertain => match client.get::<Operation>(&url).await {
            Ok(op) => op,
            Err(_) => return Err(e),
        },
        Err(e) => return Err(e),
    };
    client.settle(op, &request.key, 600).await
}
pub(crate) async fn profiles(
    session: RwSignal<Option<Principal>>,
    router: &str,
    form: &web_contract::routers::RouterEditForm,
) -> Result<web_contract::routers::RouterEditReply, ApiError> {
    let c = Client::new(session)?;
    let reply=c.response(&format!("/api/v1/routers/{}",encoded(router)),"patch",Some(form)).await?.json::<web_contract::routers::RouterEditOutcome>().await.map_err(|_|ApiError::local("The access-save reply is unknown. Reload cached configuration before another save."))?;
    if reply.code != "ok" {
        return Err(ApiError {
            status: None,
            code: reply.code,
            message: reply.message,
            uncertain: false,
        });
    }
    reply.reply.ok_or_else(|| {
        ApiError::local(
            "The access-save reply is unknown. Reload cached configuration before another save.",
        )
    })
}
