[根目录](../../CLAUDE.md) > [crates](../) > **pebble-oauth**

# pebble-oauth

## 模块职责

OAuth2 认证流程管理。处理 Google 和 Microsoft 账号的 OAuth 授权，包括授权码交换、token 刷新等。

## 入口与启动

- **入口文件**: `src/lib.rs`

## 对外接口

### 功能范围
- OAuth2 授权码流程
- Token 刷新
- Google OAuth (使用 GOOGLE_CLIENT_ID/SECRET)
- Microsoft OAuth (使用 MICROSOFT_CLIENT_ID/SECRET)

### 与 Tauri 命令层的交互
- `commands/oauth.rs` 暴露 `complete_oauth_flow` 命令
- OAuth 凭据通过 `build.rs` 从 `.env` 注入编译环境

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| oauth2 | OAuth2 协议实现 |
| reqwest | HTTP 客户端 |
| tokio | 异步运行时 |
| url | URL 处理 |
| pebble-core | Result 类型 |
| serde + serde_json | JSON 解析 |
| tracing | 日志 |

## 数据模型

- OAuth token 存储在 pebble-store 的 auth_data 模块中
- 支持 access_token 和 refresh_token

## 测试与质量

- 本 crate 无独立 `#[cfg(test)]` 模块

## 常见问题 (FAQ)

### Q: OAuth 凭据如何管理？
A: 存储在 `.env` 文件中，通过 `src-tauri/build.rs` 在编译时注入。不要将 `.env` 提交到版本库。

## 文件清单 (4 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | OAuth 入口 |
| `src/pkce.rs` | PKCE (Proof Key for Code Exchange) |
| `src/redirect.rs` | OAuth 回调重定向处理 |
| `src/tokens.rs` | Token 存储与刷新 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (4 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
