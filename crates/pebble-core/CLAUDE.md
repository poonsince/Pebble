[根目录](../../CLAUDE.md) > [crates](../) > **pebble-core**

# pebble-core

## 模块职责

Pebble 项目的共享类型库，定义所有核心数据模型、错误类型和公共 trait。这是所有其他 crate 的基础依赖。

## 入口与启动

- **入口文件**: `src/lib.rs`
- 导出: `PebbleError`, `Result`, 所有核心类型, `new_id()`, `now_timestamp()`

## 对外接口

### 公开模块
| 模块 | 说明 |
|------|------|
| `error` | `PebbleError` 枚举 + `Result<T>` 别名 |
| `traits` | 公共 trait 定义 (如 `SearchHit`) |
| `types` | 所有核心数据类型 |

### 核心类型 (推断自其他 crate 的使用)
- **Account**: 邮件账号 (id, email, display_name, color, provider, created_at, updated_at)
- **Message**: 邮件实体 (完整字段: subject, body_text, body_html_raw, from/to/cc/bcc, flags, thread_id 等)
- **Folder**: 文件夹 (id, account_id, remote_id, name, folder_type, role, parent_id 等)
- **Rule**: 自动化规则 (id, name, priority, conditions, actions, is_enabled)
- **ProviderType**: 枚举 (Imap, Gmail, Outlook)
- **FolderType**: 枚举
- **FolderRole**: 枚举 (Inbox, Sent, Trash, Drafts, Archive 等)
- **EmailAddress**: 结构 (name, address)
- **SearchHit**: 搜索结果项 (message_id, score, snippet, subject, from_address, date)

### 工具函数
- `new_id()`: 生成 UUID v4
- `now_timestamp()`: 获取当前 Unix 时间戳

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| serde + serde_json | 序列化/反序列化 |
| thiserror | 错误定义 |
| uuid | UUID v4 生成 |
| rusqlite (可选) | SQLite 类型支持 (通过 feature) |
| async-trait | 异步 trait 支持 |

## 数据模型

### PebbleError
统一错误类型，覆盖:
- 存储错误 (Storage)
- 内部错误 (Internal)
- 网络错误
- 认证错误
- 等等

### Feature Flags
- `default`: 无
- `rusqlite`: 启用 rusqlite 依赖 (pebble-store 使用)

## 测试与质量

- 本 crate 无独立测试 (测试在各消费 crate 中进行)

## 常见问题 (FAQ)

### Q: 为什么 ID 使用 UUID v4？
A: 避免中心化 ID 生成冲突，适合分布式/离线场景。

### Q: types 模块包含什么？
A: 所有与数据库行对应的结构体、枚举类型、以及业务逻辑中使用的 DTO。

## 文件清单 (4 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | 入口，重新导出 |
| `src/error.rs` | `PebbleError` 错误枚举 |
| `src/traits.rs` | 公共 trait (如 `SearchHit`) |
| `src/types.rs` | 核心数据类型 (Account/Message/Folder/Rule 等) |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (4 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
- 类型定义基于其他 crate 的使用推断
