//! Authentication middleware (stub).
//!
//! Validates a bearer token against the secret loaded from the environment.
//! The real relay will likely move to signed/expiring tokens; this stub keeps
//! the wiring in place so handlers can assume an authenticated caller.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode, header};
use axum::middleware::Next;
use axum::response::Response;

use crate::AppState;

/// 从 `Authorization: Bearer <token>` 头里提取 token。
///
/// 注意：密钥来自环境变量（见 `config::Config::auth_secret`），**绝不落仓**，
/// 也绝不写进日志——下面只比较，不打印 token 内容。
fn extract_bearer(req: &Request<Body>) -> Option<&str> {
    let value = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    // 形如 "Bearer xxx"，大小写不敏感地剥掉前缀。
    let mut parts = value.splitn(2, ' ');
    let scheme = parts.next()?;
    if !scheme.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = parts.next()?.trim();
    if token.is_empty() { None } else { Some(token) }
}

/// 恒定时间比较，避免时序侧信道泄漏密钥长度/前缀信息。
///
/// 这里手写一个简单实现，避免为 stub 引入额外依赖；正式版建议换成
/// `subtle::ConstantTimeEq` 或基于 HMAC 的签名校验。
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Axum middleware that rejects requests without a valid bearer token.
///
/// On success the request is forwarded to the next layer; otherwise a
/// `401 Unauthorized` is returned with no body.
pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 校验 token：与环境变量里的鉴权密钥做恒定时间比较。
    match extract_bearer(&req) {
        Some(token) if constant_time_eq(token.as_bytes(), state.config.auth_secret.as_bytes()) => {
            // 通过——放行到下一层。注意不要记录 token。
            Ok(next.run(req).await)
        }
        _ => {
            // 失败统一返回 401，不区分「没带 token」与「token 错」，减少探测信息。
            tracing::warn!("鉴权失败：缺失或非法 token");
            Err(StatusCode::UNAUTHORIZED)
        }
    }
}
