use super::*;
use futures_util::future::{select, Either};
use serde_json::{json, Value};
use std::time::Duration;

pub struct SfuService<'a>(pub &'a Env);

impl SfuService<'_> {
    pub async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<Value>,
    ) -> AppResult<Value> {
        let app = self
            .0
            .var("WALKIE_SFU_APP_ID")
            .map_err(|_| AppError::service_unavailable("Walkie-talkie audio is not configured."))?
            .to_string();
        let secret = self
            .0
            .secret("WALKIE_SFU_APP_SECRET")
            .map_err(|_| AppError::service_unavailable("Walkie-talkie audio is not configured."))?
            .to_string();
        self.fetch(
            method,
            &format!("https://rtc.live.cloudflare.com/v1/apps/{app}{path}"),
            &secret,
            body,
        )
        .await
    }

    async fn fetch(
        &self,
        method: Method,
        url: &str,
        secret: &str,
        body: Option<Value>,
    ) -> AppResult<Value> {
        let mut init = RequestInit::new();
        init.with_method(method);
        if let Some(v) = body {
            init.with_body(Some(v.to_string().into()));
        }
        let req = Request::new_with_init(url, &init).map_err(db_error)?;
        req.headers()
            .set("Authorization", &format!("Bearer {secret}"))
            .map_err(db_error)?;
        req.headers()
            .set("Content-Type", "application/json")
            .map_err(db_error)?;
        let controller = AbortController::default();
        let signal = controller.signal();
        // Bound outbound work so a slow provider cannot hold the room's event gate indefinitely.
        let request = Box::pin(async move {
            let mut res = Fetch::Request(req)
                .send_with_signal(&signal)
                .await
                .map_err(|_| AppError::bad_gateway("Audio service could not be reached."))?;
            let status = res.status_code();
            if !(200..300).contains(&status) {
                console_error!("walkie SFU request failed status={}", status);
                return Err(AppError::new(
                    "Audio service request failed. Reconnect audio.",
                    if status == 404 || status == 410 || status == 425 {
                        410
                    } else {
                        502
                    },
                ));
            }
            if status == 204 {
                return Ok(json!({}));
            }
            let value: Value = res
                .json()
                .await
                .map_err(|_| AppError::bad_gateway("Invalid audio service response."))?;
            if value.get("errorCode").is_some()
                || value
                    .get("tracks")
                    .and_then(Value::as_array)
                    .is_some_and(|a| a.iter().any(|t| t.get("errorCode").is_some()))
            {
                console_error!("walkie SFU operation returned resource errors");
                return Err(AppError::bad_gateway(
                    "Audio track operation failed. Reconnect audio.",
                ));
            }
            Ok(value)
        });
        match select(request, Box::pin(Delay::from(Duration::from_secs(10)))).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => {
                controller.abort();
                Err(AppError::bad_gateway(
                    "Audio service timed out. Reconnect audio.",
                ))
            }
        }
    }

    pub async fn ice_servers(&self) -> AppResult<Value> {
        // TURN is optional locally; when configured, fail rather than silently omit a broken relay.
        let Ok(key) = self.0.var("WALKIE_TURN_KEY_ID") else {
            return Ok(json!([{"urls":"stun:stun.cloudflare.com:3478"}]));
        };
        let secret = self
            .0
            .secret("WALKIE_TURN_API_TOKEN")
            .map_err(|_| AppError::service_unavailable("TURN credentials are not configured."))?
            .to_string();
        let value=self.fetch(Method::Post,&format!("https://rtc.live.cloudflare.com/v1/turn/keys/{key}/credentials/generate-ice-servers"),&secret,Some(json!({"ttl":3600}))).await?;
        value
            .get("iceServers")
            .cloned()
            .ok_or_else(|| AppError::bad_gateway("Invalid TURN response."))
    }

    pub async fn close(&self, session: &str, mids: &[String]) -> AppResult<()> {
        if mids.is_empty() {
            return Ok(());
        }
        match self
            .request(
                Method::Put,
                &format!("/sessions/{session}/tracks/close"),
                Some(json!({"force":true,"tracks":mids.iter().map(|m| json!({"mid":m})).collect::<Vec<_>>()})),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(e) if e.status == 404 || e.status == 410 || e.status == 425 => Ok(()),
            Err(e) => Err(e),
        }
    }
}
