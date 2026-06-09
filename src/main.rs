//! AI Pocket WAN relay server.
//!
//! When two peers (desktop sender + mobile/device receiver) cannot reach each
//! other directly on the LAN, this server acts as a public relay: it terminates
//! authenticated WebSocket connections and forwards signaling messages between
//! peers so a session can still be established across NAT.
//!
//! Routes:
//! - `GET /health` — liveness probe (unauthenticated).
//! - `GET /ws` — authenticated WebSocket signaling relay (stub).

mod auth;
mod config;

use std::sync::Arc;

use anyhow::{Context, Result};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use crate::config::Config;

/// Shared application state handed to every handler via [`State`].
#[derive(Debug)]
pub struct AppState {
    /// 运行期配置（含鉴权密钥，仅驻留内存）。
    pub config: Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    // 先读配置——日志级别也来自配置，所以要在初始化 tracing 之前拿到。
    let config = Config::from_env().context("加载配置失败")?;

    // 初始化 tracing：优先用进程级 RUST_LOG，否则回落到配置里的过滤级别。
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.log_filter.clone()));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let addr = config.socket_addr();
    let state = Arc::new(AppState { config });

    // 受保护的子路由：/ws 需要鉴权中间件。
    let protected = Router::new()
        .route("/ws", get(ws_handler))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    // /health 不鉴权，给 LB / k8s 探活用。
    let app = Router::new()
        .route("/health", get(health))
        .merge(protected)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!(%addr, "ai-pocket-server 启动，监听中");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("绑定监听地址失败: {addr}"))?;
    axum::serve(listener, app)
        .await
        .context("HTTP server 运行出错")?;

    Ok(())
}

/// Liveness probe. Returns `200 OK` with a static body.
async fn health() -> impl IntoResponse {
    "ok"
}

/// WebSocket signaling relay entry point.
///
/// Upgrades the connection and hands it to [`relay_socket`]. Reaching this
/// handler implies the caller already passed the auth middleware.
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(relay_socket)
}

/// 信令中继占位实现。
///
/// 目前只把收到的消息原样回显（echo），用于打通连接与鉴权链路。
/// 后续会接入「按会话 ID 配对两端 + 转发 offer/answer/candidate」的真实中继逻辑，
/// 以及与 `ai-pocket-transport` 的会话/握手协议对接。
async fn relay_socket(mut socket: WebSocket) {
    tracing::info!("ws 信令连接已建立（中继 stub）");
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                // TODO: 解析信令 -> 按 session 路由到对端，而不是回显。
                if socket.send(Message::Text(text)).await.is_err() {
                    break;
                }
            }
            Message::Binary(bin) => {
                if socket.send(Message::Binary(bin)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            // Ping/Pong 由 axum 底层自动处理，这里忽略。
            _ => {}
        }
    }
    tracing::info!("ws 信令连接已关闭");
}
