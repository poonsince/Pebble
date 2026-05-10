[根目录](../../CLAUDE.md) > [crates](../) > **pebble-mail**

# pebble-mail

## 模块职责

Pebble 的邮件引擎核心。负责 IMAP/SMTP 连接、邮件同步 (增量/全量)、Gmail/Outlook 专用同步、IMAP IDLE 实时监听、邮件解析、线程构建、邮件调和 (reconcile) 等。

## 入口与启动

- **入口文件**: `src/lib.rs`
- 公开导出: `GmailSyncWorker`, `OutlookSyncWorker`, `ImapProvider`, `GmailProvider`, `OutlookProvider`, `SyncWorker`, `ConnectionSecurity`, `ImapConfig`, `SmtpConfig`, `ProxyConfig`, `RealtimePollPolicy`, `SyncTrigger`

## 对外接口

### 公开模块
| 模块 | 说明 |
|------|------|
| `imap` | IMAP 连接与操作 (ImapProvider, ImapConfig) |
| `smtp` | SMTP 发送 |
| `sync` | 通用同步引擎 (SyncWorker, SyncConfig, SyncProgress) |
| `idle` | IMAP IDLE 实时监听 |
| `gmail_sync` | Gmail API 专用同步 (GmailSyncWorker) |
| `outlook_sync` | Outlook API 专用同步 (OutlookSyncWorker) |
| `reconcile` | 邮件状态调和 |
| `parser` | 邮件解析 (mail-parser) |
| `thread` | 邮件线程构建 |
| `provider` | 邮件提供商适配 (GmailProvider, ImapMailProvider, OutlookProvider) |
| `realtime_policy` | 实时同步策略 |
| `backoff` | 退避策略 |

### 关键结构
- **SyncWorker**: 通用同步工作线程，支持增量/全量同步
- **ImapProvider**: IMAP 邮件操作提供者
- **GmailSyncWorker**: Gmail API 同步 (支持 labels)
- **OutlookSyncWorker**: Microsoft Graph API 同步
- **ConnectionSecurity**: TLS 安全配置
- **SyncProgress**: 同步进度报告

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| async-imap | IMAP 协议客户端 |
| lettre | SMTP 邮件发送 |
| mail-parser | RFC 邮件解析 |
| rustls + tokio-rustls | TLS |
| webpki-roots | Web PKI 根证书 |
| pebble-core | 共享类型 |
| pebble-store | 数据持久化 |
| tokio + futures | 异步 |
| reqwest | HTTP (API 调用) |
| tokio-socks | SOCKS 代理 |
| utf7-imap | IMAP UTF7 编码 |

## 数据模型

### 同步流程
1. 建立 IMAP/API 连接
2. 列出文件夹
3. 选择目标文件夹
4. 获取 UIDVALIDITY / MODSEQ
5. 增量/全量同步邮件
6. 调和本地与远程状态
7. 提交到 pebble-store

### SyncError
统一同步错误类型

## 测试与质量

- 本 crate 无独立 `#[cfg(test)]` 模块 (集成测试在 pebble-store 和 src-tauri 中进行)

## 常见问题 (FAQ)

### Q: 支持哪些邮件提供商？
A: 标准 IMAP、Gmail (IMAP + API)、Outlook (IMAP + Graph API)。

### Q: 实时同步如何实现？
A: 通过 IMAP IDLE 命令监听服务器推送，结合轮询退避策略。

## 文件清单 (13 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | 入口，公开导出 |
| `src/imap.rs` | IMAP 连接与操作 |
| `src/smtp.rs` | SMTP 发送 |
| `src/sync.rs` | 通用同步引擎 |
| `src/idle.rs` | IMAP IDLE 实时监听 |
| `src/gmail_sync.rs` | Gmail API 同步 |
| `src/outlook_sync.rs` | Outlook/Graph API 同步 |
| `src/reconcile.rs` | 状态调和 |
| `src/parser.rs` | 邮件解析 (mail-parser) |
| `src/thread.rs` | 线程构建 |
| `src/provider/mod.rs` | 提供商模块索引 |
| `src/provider/gmail.rs` | Gmail 提供商适配 |
| `src/provider/imap_provider.rs` | IMAP 提供商适配 |
| `src/provider/outlook.rs` | Outlook 提供商适配 |
| `src/realtime_policy.rs` | 实时同步策略 |
| `src/backoff.rs` | 退避策略 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (13 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
- 识别 12 个公开模块
