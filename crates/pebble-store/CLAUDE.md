[根目录](../../CLAUDE.md) > [crates](../) > **pebble-store**

# pebble-store

## 模块职责

Pebble 的数据持久化层。基于 SQLite + r2d2 连接池，提供所有实体的 CRUD 操作、数据库迁移、搜索待处理操作管理、云备份存储等。

## 入口与启动

- **入口文件**: `src/lib.rs`
- **关键 API**:
  - `Store::open(path)`: 打开文件数据库 (含迁移)
  - `Store::open_in_memory()`: 打开内存数据库 (测试用)
  - `quick_check()`: 数据库完整性检查
  - `vacuum()`: 空间回收

### 连接池配置
- **写池**: 最大 1 连接 (保证写一致性)
- **读池**: 最大 4 连接 (并发读)
- PRAGMA: `foreign_keys=ON`, `busy_timeout=5000`, `synchronous=NORMAL`

## 对外接口

### 数据模块 (每个模块对应一组实体操作)

| 模块 | 说明 |
|------|------|
| `accounts` | 账号 CRUD、代理配置、SyncState 枚举 |
| `messages` | 邮件查询、插入、更新、软删除 |
| `folders` | 文件夹管理 |
| `labels` | 标签 (Gmail labels) |
| `kanban` | 看板卡片管理 |
| `snooze` | 延迟发送邮件 |
| `trusted_senders` | 信任发件人列表 |
| `rules` | 自动化规则 CRUD |
| `contacts` | 联系人 |
| `attachments` | 附件记录 |
| `pending_ops` | 待处理操作队列 |
| `search_pending` | 搜索索引待处理操作 |
| `sync_failures` | 同步失败记录 |
| `translate_config` | 翻译配置 |
| `secure_user_data` | 安全用户数据 (加密存储) |
| `cloud_sync` | WebDAV 云备份元数据 |
| `auth_data` | 认证凭据 |
| `migrations` | 数据库迁移 |

### 异步方法
- `with_read_async`: 在 blocking thread 上执行读操作
- `with_write_async`: 在 blocking thread 上执行写操作

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| rusqlite | SQLite 驱动 |
| r2d2 + r2d2_sqlite | 连接池 |
| pebble-core | 共享类型 |
| tokio | 异步运行时 |
| reqwest | 网络请求 (云备份) |
| uuid | ID 生成 |

## 数据模型

### 核心实体关系
```
Account (1) --> (N) Folder
Account (1) --> (N) Message
Message (N) --> (N) Folder (通过关联表)
Message (N) --> (N) Label
Message (1) --> (N) Snooze
Message (1) --> (N) KanbanCard
```

### 迁移系统
- 自动在 `Store::open()` 时执行
- 定义在 `migrations` 模块

## 测试与质量

- 内置单元测试: `test_open_in_memory`, `test_account_crud`, `test_folder_crud`, `test_message_insert_and_query`
- 使用 `tempfile` crate 做临时文件测试

## 常见问题 (FAQ)

### Q: 为什么使用连接池而不是单连接？
A: 支持读写分离，多个读请求可并发执行，写操作保证串行一致性。

### Q: 软删除如何工作？
A: Message 有 `is_deleted` 和 `deleted_at` 字段，查询时过滤，定期 VACUUM 回收空间。

## 文件清单 (14 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | Store 主体、连接池、初始化 |
| `src/accounts.rs` | 账号操作 |
| `src/messages.rs` | 邮件操作 |
| `src/folders.rs` | 文件夹操作 |
| `src/migrations.rs` | 数据库迁移 |
| `src/labels.rs` | 标签操作 |
| `src/kanban.rs` | 看板操作 |
| `src/snooze.rs` | 延迟发送操作 |
| `src/rules.rs` | 规则操作 |
| `src/pending_ops.rs` | 待处理操作队列 |
| `src/search_pending.rs` | 搜索索引待处理 |
| `src/contacts.rs` | 联系人 |
| `src/attachments.rs` | 附件记录 |
| `src/trusted_senders.rs` | 信任发件人 |
| `src/sync_failures.rs` | 同步失败记录 |
| `src/translate_config.rs` | 翻译配置 |
| `src/secure_user_data.rs` | 加密用户数据 |
| `src/cloud_sync.rs` | 云备份元数据 |
| `src/auth_data.rs` | 认证凭据 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (14 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
- 识别 19 个子模块
