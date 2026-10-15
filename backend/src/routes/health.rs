use worker::*;

pub fn health(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let environment = ctx
        .env
        .var("ENVIRONMENT")
        .map(|secret| secret.to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    Response::from_json(&serde_json::json!({
        "status": "ok",
        "release": env!("CARGO_PKG_VERSION"),
        "environment": environment,
    }))
}

