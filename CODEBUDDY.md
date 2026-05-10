# CODEBUDDY.md This file provides guidance to CodeBuddy when working with code in this repository.

## 常用命令

```bash
# 开发
pnpm dev              # 启动完整 Tauri 开发模式（前端热重载 + Rust 后端）
pnpm dev:frontend     # 仅启动前端 Vite 开发服务器 (端口 1420)

# 构建
pnpm build:frontend   # tsc 类型检查 + vite build
pnpm build            # 构建完整 Tauri 应用（平台自动检测）
pnpm build:linux      # Linux AppImage
pnpm build:macos      # macOS .app + .dmg
pnpm build:windows    # Windows NSIS 安装包

# Rust 检查
cargo check           # Rust 编译检查（最快反馈）
cargo clippy          # Rust lint 检查
cargo test            # 运行所有 Rust 测试（pebble-crypto 的测试标记为 #[ignore]，需要 OS 密钥链）
cargo test -p pebble-store  # 运行单个 crate 的测试

# 前端测试
pnpm test             # 运行所有 Vitest 测试（78 个测试文件）
pnpm test -- -t "InboxView"  # 按名称过滤测试

# 前端类型检查
npx tsc --noEmit      # TypeScript 编译检查
```

## 架构概述

Pebble 是一个基于 **Tauri v2** 的跨平台桌面邮件客户端，**Rust 后端**处理所有业务逻辑（邮件同步、搜索、加密、规则引擎），**React 19 + TypeScript 前端**通过 **Tauri IPC (`invoke`)** 调用后端命令。

### 项目结构

```
Pebble/
├── src-tauri/           # Rust Tauri 后端（42 个 .rs 文件）
│   ├── src/lib.rs       # 应用入口：注册 ~100 个 Tauri 命令、启动任务
│   ├── src/main.rs      # 仅调用 pebble_lib::run()
│   ├── src/state.rs     # AppState（store, search, crypto, sync_handles 等）
│   ├── src/commands/    # 25 个命令模块（mail, accounts, search, sync 等）
│   ├── src/realtime/    # 实时同步触发器（IMAP IDLE / Gmail / Outlook push）
│   └── src/snooze_watcher.rs  # 延时消息到期监控
├── crates/              # 9 个 Rust crate（60 个 .rs 文件）
│   ├── pebble-core/     # 共享类型（Account/Message/Folder/Rule）、错误定义、trait
│   ├── pebble-store/    # SQLite 数据层（r2d2 连接池：1写+4读）、CRUD、迁移
│   ├── pebble-mail/     # 邮件引擎：IMAP 同步、Gmail API、Outlook API、SMTP 发送、线程化
│   ├── pebble-search/   # Tantivy 全文搜索（自定义分词器：body 用 BODY_TOKENIZER、短字段用 NGRAM_TOKENIZER）
│   ├── pebble-rules/    # 规则引擎：JSON 条件 + 动作匹配（自动标签、标记已读）
│   ├── pebble-translate/# 翻译服务（DeepL / DeepLX / Generic API / LLM）
│   ├── pebble-crypto/   # AES-256-GCM 加密 + OS 密钥链（keyring crate）
│   ├── pebble-oauth/    # OAuth2 PKCE 流程（Google / Microsoft）
│   └── pebble-privacy/  # HTML 消毒（ammonia）+ 邮件追踪像素拦截
├── src/                 # React 前端（121 个文件）
│   ├── main.tsx         # 入口（QueryClient + i18n + showMainWindow）
│   ├── App.tsx          # ErrorBoundary + Layout
│   ├── app/Layout.tsx   # 状态驱动视图路由（无 React Router，使用 Zustand activeView）
│   ├── lib/api.ts       # Tauri IPC 桥接层（所有 invoke 调用集中在此）
│   ├── lib/ipc-types.ts # TypeScript 类型定义（镜像 Rust 结构体）
│   ├── features/        # 功能视图：inbox, compose, kanban, search, settings 等
│   ├── components/      # 通用 UI 组件（TitleBar, Sidebar, MessageList 等）
│   ├── stores/          # 8 个 Zustand store（ui, mail, compose, kanban 等）
│   ├── hooks/           # TanStack Query hooks（queries/ + mutations/）
│   └── locales/         # 国际化（en.json + zh.json）
└── tests/               # 78 个 Vitest 测试文件
```

### 关键架构概念

1. **数据流**: `api.ts` (invoke) → `src-tauri/commands/` → pebble-store / pebble-mail / pebble-search 等 crate

2. **AppState**（`state.rs`）: 持有 `Store`（SQLite）、`TantivySearch`、`CryptoService` 的 `Arc` 指针，以及同步句柄 Map。通过 `app.manage()` 注入 Tauri。

3. **状态驱动导航**: 前端不使用 React Router。`Layout.tsx` 根据 `uiStore.activeView` 值（`"inbox" | "kanban" | "settings" | "search" | "snoozed" | "starred" | "compose"`）条件渲染对应视图，所有视图通过 `React.lazy()` + `Suspense` 懒加载。

4. **后台工作线程**（`lib.rs`）:
   - `snooze_watcher`: 监控延时邮件到期
   - `run_pending_mail_ops_worker`: 处理待发送邮件队列
   - `resume_all_syncs`: 启动时恢复所有账号同步
   - 搜索索引重建（`spawn_blocking`）

5. **数据模型**: 所有 ID 使用 UUID v4。核心实体：Account（邮件账号）、Message（邮件，含 soft-delete）、Folder（邮箱文件夹，含系统角色）、Rule（自动化规则，JSON 条件+动作）

6. **存储**:
   - SQLite: `pebble-store`（r2d2 连接池）
   - Tantivy: `pebble-search`（独立索引目录）
   - 附件: 文件系统缓存
   - 敏感数据: OS 密钥链 + AES-256-GCM 加密

### 关键依赖

- **后端**: tokio（异步运行时）、rusqlite（bundled SQLite）、tantivy 0.22（全文搜索）、async-imap + lettre（邮件协议）、aes-gcm + keyring（加密）、ammonia + lol_html（隐私保护）
- **前端**: React 19、TanStack React Query（服务端状态）、Zustand 5（客户端状态）、TailwindCSS v4（样式）、i18next（国际化）、@tauri-apps/api（IPC）、@dnd-kit（看板拖拽）、@tiptap（富文本编辑）
