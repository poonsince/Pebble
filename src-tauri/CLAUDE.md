[根目录](../../CLAUDE.md) > **src-tauri**

# src-tauri (Tauri 后端核心)

## 模块职责

Tauri v2 应用的主入口。负责：
- 初始化所有 crate 服务 (store, search, crypto)
- 注册 Tauri 命令处理器 (invoke_handler)
- 管理系统托盘图标与菜单
- 启动后台工作线程 (snooze watcher, sync resume, pending mail ops, search reindex)
- 处理 deep link (mailto: 协议)
- 多平台通知管理 (Windows/Linux/macOS)

## 入口与启动

- **入口文件**: `src/lib.rs`
  - `run()` 函数是 Tauri 应用主入口
  - `#[cfg_attr(mobile, tauri::mobile_entry_point)]` 支持移动端编译
- **构建脚本**: `build.rs` — 从 `.env` 读取 OAuth 凭据并注入编译环境
- **配置文件**: `tauri.conf.json` — CSP、窗口尺寸、插件配置

### 启动流程

1. 创建系统托盘 (tray icon)
2. 解析数据目录路径 (`app_data_dir`)
3. 初始化 `pebble_store::Store` (SQLite + migrations)
4. 初始化 `pebble_search::TantivySearch` (索引目录)
5. 初始化 `pebble_crypto::CryptoService`
6. 注册 `AppState` 到 Tauri 管理状态
7. 启动 snooze watcher 后台线程
8. 启动搜索索引重建任务 (如果需要)
9. 启动自动同步恢复任务
10. 启动待发送邮件队列处理任务
11. 注册所有 Tauri 命令

## 对外接口 (Tauri 命令)

所有命令通过 `tauri::generate_handler![]` 注册。按模块分类:

### 账户管理 (`commands/accounts.rs`)
- `add_account`, `list_accounts`, `delete_account`, `update_account`
- `get_account_proxy`, `update_account_proxy`, `test_imap_connection`, `test_account_connection`

**输入类型:**
- `AddAccountRequest`: `{ email, display_name, provider, imap_host, imap_port, smtp_host, smtp_port, username, password, imap_security, smtp_security, proxy_host?, proxy_port? }` (camelCase)
- `TestConnectionRequest`: `{ imap_host, imap_port, imap_security, proxy_host?, proxy_port?, username?, password? }`

### 邮件查询 (`commands/messages/`)
- `list_messages`, `get_message`, `get_messages_batch` (query)
- `get_rendered_html`, `get_message_with_html` (rendering)
- `update_message_flags` (flags)
- `archive_message`, `delete_message`, `restore_message`, `empty_trash`, `move_to_folder` (lifecycle)

### 同步 (`commands/sync_cmd.rs`)
- `start_sync`, `stop_sync`, `trigger_sync`, `resume_all_syncs`
- `set_realtime_preference`, `reindex_search`

### 搜索 (`commands/search.rs`)
- `search_messages`, `advanced_search`

### 撰写与草稿 (`commands/compose.rs`, `commands/drafts.rs`)
- `send_email`, `stage_compose_attachment`
- `save_draft`, `delete_draft`

### 标签与规则 (`commands/labels.rs`, `commands/rules.rs`)
- `list_labels`, `add_message_label`, `remove_message_label`, `get_message_labels`
- `create_rule`, `list_rules`, `update_rule`, `delete_rule`

### 延迟发送 (`commands/snooze.rs`)
- `snooze_message`, `unsnooze_message`, `list_snoozed`

### 看板 (`commands/kanban.rs`)
- `move_to_kanban`, `list_kanban_cards`, `remove_from_kanban`
- `list_kanban_context_notes`, `set_kanban_context_note`, `merge_kanban_context_notes`

### 翻译 (`commands/translate.rs`)
- `translate_text`, `get_translate_config`, `save_translate_config`, `test_translate_connection`

### 云备份 (`commands/cloud_sync.rs`)
- `test_webdav_connection`, `backup_to_webdav`, `preview_webdav_backup`, `restore_from_webdav`

### 健康与诊断 (`commands/health.rs`, `commands/diagnostics.rs`)
- `health_check`, `check_for_update`, `open_external_url`
- `read_app_log`

### 通知 (`commands/notifications.rs`)
- `set_notifications_enabled`, `get_notification_status`, `show_test_notification`, `clear_notification_attention`

### OAuth (`commands/oauth.rs`)
- `complete_oauth_flow`, proxy 管理命令

### 其他
- 联系人 (`commands/contacts.rs`): `search_contacts` — 按账号/关键词/数量搜索联系人
- 附件 (`commands/attachments.rs`): `list_attachments`, `get_attachment_path`, `download_attachment` (含 `attachment:download-progress` 事件)
- 线程 (`commands/threads.rs`): `list_threads`, `list_thread_messages`
- 信任发件人 (`commands/trusted_senders.rs`): `trust_sender`, `list_trusted_senders`, `remove_trusted_sender`
- 用户数据 (`commands/user_data.rs`): 邮件模板 (`list/save/delete`) + 签名 (`get/set`)
- 批量操作 (`commands/batch.rs`): `batch_archive`, `batch_delete`, `batch_mark_read`, `batch_star`
- 文件夹计数 (`commands/folder_counts.rs`): `get_folder_unread_counts`
- 高级搜索 (`commands/advanced_search.rs`): `advanced_search` — 输入 `AdvancedSearchQuery` (含 text/from/to/subject/date范围/附件/文件夹)
- 网络代理 (`commands/network.rs`): `get_global_proxy`, `update_global_proxy`
- 索引管理 (`commands/index.rs`): 索引相关操作
- 待发送邮件 (`commands/pending_mail_ops.rs`): `get_pending_mail_ops_summary`, `list_pending_mail_ops`

**所有命令输出:** 返回 `Result<T, PebbleError>`，错误通过 `PebbleError` 枚举序列化 (Validation/Internal/Network/OAuth/UnsupportedProvider/Translate 等变体)。

**辅助类型:**
- `UpdateInfo`: `{ latest_version, release_url, is_newer }`
- `AppLogSnapshot`: `{ path, content, truncated }`
- `NotificationStatus`: `{ enabled, attention_active, platform, app_id }`
- `PendingMailOpsSummaryResponse`: `{ pending_count, in_progress_count, failed_count, total_active_count, last_error?, updated_at }`
- `PendingMailOpResponse`: `{ id, account_id, message_id, op_type, status, attempts, last_error?, created_at, updated_at, next_retry_at? }`
- `EmailTemplate`: `{ id, name, subject, body, created_at }` (camelCase)
- `SaveEmailTemplateRequest`: `{ name, subject, body }` (camelCase)
- `KanbanColumn`: 看板列枚举
- `TrustType`: 信任类型枚举
- `PrivacyMode`: 隐私模式枚举
- `AccountProxyMode`: 代理模式枚举
- `AccountProxySetting`: `{ mode, host?, port? }`
- `HttpProxyConfig`: `{ host, port }`

## 文件清单

| 依赖 | 用途 |
|------|------|
| tauri v2 | 桌面应用框架 |
| pebble_core | 共享类型 |
| pebble_store | SQLite 数据层 |
| pebble_mail | IMAP/SMTP 邮件引擎 |
| pebble_search | Tantivy 全文搜索 |
| pebble_crypto | AES-GCM 加密 |
| pebble_rules | 规则引擎 |
| pebble_translate | 翻译服务 |
| pebble_oauth | OAuth2 集成 |
| pebble_privacy | 隐私保护 |
| tracing + tracing-appender | 日志系统 |
| tokio | 异步运行时 |
| reqwest | HTTP 客户端 |
| lettre | SMTP 发送邮件 |

### 平台特定依赖
- **Windows**: `tauri-winrt-notification`, `windows-registry`
- **Linux**: `notify-rust`
- **macOS**: `mac-notification-sys`
- **所有桌面**: `tauri-plugin-single-instance` (含 deep-link)

## 数据模型

### AppState (`state.rs`)
```rust
pub struct AppState {
    pub store: Arc<Store>,
    pub search: Arc<TantivySearch>,
    pub crypto: Arc<CryptoService>,
    pub sync_handles: Mutex<HashMap<String, SyncHandle>>,
    pub snooze_stop_tx: mpsc::Sender<()>,
    pub attachments_dir: PathBuf,
    pub notifications_enabled: Arc<AtomicBool>,
    pub notification_attention_active: Arc<AtomicBool>,
}
```

### SyncHandle
- `stop_tx`: watch channel 停止同步
- `trigger_tx`: 手动触发同步
- `task`: tokio task handle

### 事件常量 (`events.rs`)
- 定义前后端通信的事件名称

### 实时同步 (`realtime.rs`)
- `SyncTrigger`: 触发同步的信号类型

## 测试与质量

- 内置测试: `src/lib.rs` 中的 `startup_timing_tests` 模块
- 测试启动时序计算逻辑

## 常见问题 (FAQ)

### Q: 为什么启动时有搜索索引重建？
A: 当 schema 变更或 SQLite/Tantivy 计数不匹配时，会在后台触发全量重建，不会阻塞主窗口。

### Q: 邮件同步是实时的吗？
A: 支持 IMAP IDLE (实时) 和轮询两种模式，可通过 `set_realtime_preference` 切换。

### Q: 数据存在哪里？
A: SQLite 数据库在 `app_data_dir/db/pebble.db`，搜索索引在 `app_data_dir/search_index/`。

## 相关文件清单

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | 主入口、启动流程、命令注册 |
| `src/state.rs` | AppState 定义 |
| `src/commands/mod.rs` | 命令模块索引 (30 个子模块) |
| `src/events.rs` | 事件名称常量 |
| `src/realtime.rs` | 实时同步触发类型 |
| `src/snooze_watcher.rs` | 延迟邮件监控 |
| `src/account_colors.rs` | 账号颜色管理 |
| `build.rs` | 构建脚本 (OAuth 凭据注入) |
| `tauri.conf.json` | Tauri 配置 |
| `Cargo.toml` | crate 依赖声明 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全所有 30 个 Tauri 命令模块的输入输出类型参考
- 补充辅助类型定义 (UpdateInfo, AppLogSnapshot, NotificationStatus, PendingMailOpsSummary 等)
- 补充命令模块枚举值列表 (KanbanColumn, TrustType, PrivacyMode, AccountProxyMode 等)

### 2026-05-06 22:10:36
- 初始模块文档生成
- 识别 30 个命令子模块
- 记录完整启动流程与 AppState 结构
