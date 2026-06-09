# ai-pocket-server

WAN relay server for **AI Pocket**, a screen-cast / capture hardware product
(desktop sender, mobile receivers, device firmware). Built in Rust on
[`axum`](https://github.com/tokio-rs/axum).

## Role

AI Pocket prefers a direct LAN path between the desktop sender and the
receiver (mobile app or hardware device). When that direct path is
unreachable — peers on different networks, symmetric NAT, carrier-grade NAT —
this server steps in as a **public relay**:

- **WAN relay** — terminates connections from both peers on a publicly
  reachable address and forwards traffic between them.
- **NAT traversal fallback** — provides the rendezvous point used to exchange
  signaling (offer / answer / ICE-style candidates) when hole punching fails.
- **Authentication** — every relay connection must present a valid bearer
  **token**; unauthenticated callers are rejected with `401`.

> The H.264 media path is negotiated by the peers; this server relays
> signaling and (in later iterations) can fall back to relaying the media
> stream itself when no direct path exists.

### Endpoints

| Method | Path      | Auth | Purpose                                        |
| ------ | --------- | ---- | ---------------------------------------------- |
| `GET`  | `/health` | no   | Liveness probe.                                |
| `GET`  | `/ws`     | yes  | WebSocket signaling relay (currently a stub).  |

The `/ws` handler is wired through the auth middleware and currently echoes
messages back — the session-pairing and forwarding logic is the next step and
will integrate with `ai-pocket-transport`.

## Configuration

All configuration comes from **environment variables** — no secret is ever
committed. Copy `.env.example` to `.env` and fill in real values (the `.env`
file is git-ignored).

| Variable                | Default   | Required | Description                                 |
| ----------------------- | --------- | -------- | ------------------------------------------- |
| `AI_POCKET_BIND`        | `0.0.0.0` | no       | Listen IP address.                          |
| `AI_POCKET_PORT`        | `8080`    | no       | Listen port.                                |
| `AI_POCKET_AUTH_SECRET` | —         | **yes**  | Bearer token secret for auth (no default).  |
| `AI_POCKET_LOG`         | `info`    | no       | Log filter (`RUST_LOG` syntax).             |

## Running

```sh
# Provide configuration via the environment.
export AI_POCKET_BIND=0.0.0.0
export AI_POCKET_PORT=8080
export AI_POCKET_AUTH_SECRET="$(openssl rand -hex 32)"

# Run.
cargo run --release

# Verify.
curl http://localhost:8080/health        # -> ok

# Authenticated WebSocket (needs a ws client; token must equal the secret):
#   Authorization: Bearer <AI_POCKET_AUTH_SECRET>
```

## Workspace dependencies: path (dev) vs. git tag (release)

This repo depends on two shared crates from the `ai-pocket-core` repo:

- `ai-pocket-core`
- `ai-pocket-transport`

During **polyrepo development** they are consumed as **path dependencies**
pointing at the sibling `ai-pocket-core` checkout
(`../core/crates/core`, `../core/crates/transport`).

At **release** time, switch each to a **pinned git-tag dependency**. Every
path entry in `Cargo.toml` carries a comment with its release form, e.g.:

```toml
# 发布期改用锁定版：ai-pocket-core = { git = "https://github.com/OpenLoaf/ai-pocket-core", tag = "v0.1.0" }
ai-pocket-core = { path = "../core/crates/core" }
```

Replace `OpenLoaf` with the real GitHub org and bump the tag to match
the released version (all shared crates are kept at the same version).

## License

Dual-licensed under MIT or Apache-2.0. License files are a follow-up TODO.
