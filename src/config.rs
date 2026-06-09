//! Runtime configuration loaded from environment variables.
//!
//! All values come from the process environment so that no secret is ever
//! committed to the repository. See `.env.example` for the full list.

use std::env;
use std::net::IpAddr;

use anyhow::{Context, Result};

/// Server runtime configuration.
///
/// Constructed via [`Config::from_env`]. Networking knobs fall back to sane
/// defaults; the auth secret is required and has no default.
#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址（IP），默认 0.0.0.0 以便容器/公网中继场景下对外可达。
    pub bind: IpAddr,
    /// 监听端口，默认 8080。
    pub port: u16,
    /// 鉴权密钥：用于校验客户端 token，**只允许**来自环境变量，绝不落仓。
    pub auth_secret: String,
    /// tracing 日志过滤级别（RUST_LOG 兼容语法），默认 "info"。
    pub log_filter: String,
}

impl Config {
    /// Build a [`Config`] from process environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if a value is present but malformed, or if the
    /// required auth secret is missing.
    pub fn from_env() -> Result<Self> {
        // 监听地址：缺省 0.0.0.0（外网中继需要对公网暴露，部署时由防火墙/网关收口）。
        let bind = match env::var("AI_POCKET_BIND") {
            Ok(v) => v
                .parse::<IpAddr>()
                .with_context(|| format!("AI_POCKET_BIND 不是合法 IP: {v}"))?,
            Err(_) => IpAddr::from([0, 0, 0, 0]),
        };

        // 端口：缺省 8080。
        let port = match env::var("AI_POCKET_PORT") {
            Ok(v) => v
                .parse::<u16>()
                .with_context(|| format!("AI_POCKET_PORT 不是合法端口: {v}"))?,
            Err(_) => 8080,
        };

        // 鉴权密钥：必填。没有默认值——缺失即报错，杜绝「忘了配密钥就裸跑」。
        let auth_secret = env::var("AI_POCKET_AUTH_SECRET").context(
            "缺少必填环境变量 AI_POCKET_AUTH_SECRET（鉴权密钥，来自环境变量，绝不写进仓）",
        )?;
        if auth_secret.trim().is_empty() {
            anyhow::bail!("AI_POCKET_AUTH_SECRET 不能为空");
        }

        // 日志级别：缺省 info。
        let log_filter = env::var("AI_POCKET_LOG").unwrap_or_else(|_| "info".to_string());

        Ok(Self {
            bind,
            port,
            auth_secret,
            log_filter,
        })
    }

    /// Socket address (`bind:port`) this server should listen on.
    pub fn socket_addr(&self) -> std::net::SocketAddr {
        std::net::SocketAddr::new(self.bind, self.port)
    }
}
