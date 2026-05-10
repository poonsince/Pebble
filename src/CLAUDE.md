[根目录](../../CLAUDE.md) > **src (前端)**

# Frontend (React + TypeScript)

## 模块职责

Pebble 的前端界面。基于 React 19 + TypeScript 的单页应用，通过 Tauri IPC 与 Rust 后端通信。提供收件箱、邮件详情、撰写、看板、设置、搜索、延迟邮件、星标邮件等视图。

## 入口与启动

- **入口文件**: `src/main.tsx`
  - 初始化 React Query Provider
  - 初始化 i18n
  - 渲染 App 组件
  - 启动时间日志
- **启动流程**: `main.tsx` -> `App.tsx` -> `Layout.tsx` -> 各视图组件

## 对外接口

### 视图组件 (features/)
| 视图 | 路径 | 说明 |
|------|------|------|
| 收件箱 | `features/inbox/` | 邮件列表与详情 |
| 撰写 | `features/compose/` | 写邮件界面 |
| 看板 | `features/kanban/` | 邮件看板视图 |
| 设置 | `features/settings/` | 应用设置页 |
| 搜索 | `features/search/` | 邮件搜索页 |
| 延迟邮件 | `features/snoozed/` | 已延迟邮件列表 |
| 星标邮件 | `features/starred/` | 已星标邮件列表 |
| 命令面板 | `features/command-palette/` | 全局命令面板 (Ctrl+K) |

### 通用组件 (components/)
| 组件 | 说明 |
|------|------|
| `TitleBar` | 自定义标题栏 (无边框窗口) |
| `Sidebar` | 侧边导航栏 |
| `StatusBar` | 底部状态栏 |
| `ComposeFAB` | 浮动撰写按钮 |
| `ToastContainer` | Toast 通知容器 |
| `ConfirmDialog` | 确认对话框 |

### 状态管理 (stores/)
| Store | 说明 |
|-------|------|
| `ui.store` | UI 状态 (当前视图、主题等) |
| `compose.store` | 撰写状态 |
| `confirm store` | 确认对话框状态 |
| `command store` | 命令面板状态与注册 |
| `kanban store` | 看板卡片数据 |

### Hooks (hooks/)
| Hook | 说明 |
|------|------|
| `useKeyboard` | 全局键盘快捷键 |
| `useNetworkStatus` | 网络状态监听 |

### 工具库 (lib/)
| 模块 | 说明 |
|------|------|
| `query-client` | React Query 客户端配置 |
| `showMainWindow` | 显示主窗口 |
| `startupTiming` | 启动时间日志 |
| `i18n` | 国际化初始化 |
| `api` | Tauri API 封装 |

## 文件清单

前端共 111 个源文件，分布如下:

| 目录 | 文件数 | 说明 |
|------|--------|------|
| `src/app/` | 8 | 应用级逻辑与 hooks (lazyViewPreload, 各种 useXxx hooks) |
| `src/components/` | 18 | 通用 UI 组件 |
| `src/features/` | 33 | 功能视图组件 (9 个子功能) |
| `src/hooks/` | 6 | 通用 hooks |
| `src/hooks/mutations/` | 5 | 数据变更 hooks |
| `src/hooks/queries/` | 14 | 数据查询 hooks |
| `src/lib/` | 15 | 工具库与底层逻辑 |
| `src/locales/` | 2 | 国际化 (en, zh) |
| `src/stores/` | 8 | Zustand 状态管理 |
| `src/styles/` | 2 | 样式 |
| 根级 | 3 | App.tsx, main.tsx, vite-env.d.ts |

### `src/lib/` 详解

| 文件 | 大小 | 说明 |
|------|------|------|
| `api.ts` | 22.3 KB | Tauri IPC 通信封装层，导出所有后端命令的 TypeScript 包装函数 |
| `ipc-types.ts` | 10.3 KB | IPC 请求/响应类型定义 |
| `accountColors.ts` | 2.7 KB | 账号颜色分配逻辑 |
| `folderAggregation.ts` | 3.5 KB | 多账号文件夹聚合逻辑 |
| `sanitizeHtml.ts` | 4.3 KB | HTML 消毒 (DOMPurify 封装) |
| `sanitizeFilename.ts` | 1.2 KB | 文件名安全处理 |
| `privacyMode.ts` | 952 B | 隐私模式控制 |
| `templates.ts` | 918 B | 邮件模板工具 |
| `language.ts` | 839 B | 语言检测工具 |
| `signatures.ts` | 613 B | 邮件签名工具 |
| `startupTiming.ts` | 660 B | 启动时间日志 |
| `i18n.ts` | 475 B | 国际化初始化 |
| `extractErrorMessage.ts` | 376 B | 错误消息提取 |
| `showMainWindow.ts` | 322 B | 显示主窗口 |
| `query-client.ts` | 252 B | React Query 客户端配置 |

### `src/lib/api.ts` 详解

Tauri IPC 通信的封装层，共 617 行。所有函数通过 `invoke` 调用 Rust 后端命令，类型从 `./ipc-types` 重新导出。

#### API 模块分类

| 模块 | 主要函数 |
|------|----------|
| **Account** | `addAccount`, `listAccounts`, `deleteAccount`, `updateAccount`, `getAccountProxy`, `testImapConnection`, `testAccountConnection` |
| **OAuth** | `completeOAuthFlow`, `getOauthAccountProxy`, `updateOauthAccountProxy` |
| **Folder** | `listFolders`, `getFolderUnreadCounts` |
| **Message** | `listMessages`, `getMessage`, `getMessagesBatch`, `getRenderedHtml`, `updateMessageFlags`, `archiveMessage`, `deleteMessage`, `moveToFolder` |
| **Thread** | `listThreads`, `listThreadMessages` |
| **Search** | `searchMessages`, `advancedSearch` |
| **Sync** | `startSync`, `stopSync`, `triggerSync`, `setRealtimePreference`, `reindexSearch` |
| **Compose** | `sendEmail`, `stageComposeAttachment` |
| **Drafts** | `saveDraft`, `deleteDraft` |
| **Snooze** | `snoozeMessage`, `unsnoozeMessage`, `listSnoozed` |
| **Kanban** | `moveToKanban`, `listKanbanCards`, `removeFromKanban`, `setKanbanContextNote` |
| **Rules** | `createRule`, `listRules`, `updateRule`, `deleteRule` |
| **Labels** | `listLabels`, `addMessageLabel`, `removeMessageLabel`, `getMessageLabels`, `getMessageLabelsBatch` |
| **Translate** | `translateText`, `getTranslateConfig`, `saveTranslateConfig`, `testTranslateConnection` |
| **Contacts** | `searchContacts` |
| **Attachments** | `listAttachments`, `getAttachmentPath`, `downloadAttachment` |
| **Batch** | `batchArchive`, `batchDelete`, `batchMarkRead`, `batchStar` |
| **Cloud Sync** | `testWebdavConnection`, `backupToWebdav`, `previewWebdavBackup`, `restoreFromWebdav` |
| **Network** | `getGlobalProxy`, `updateGlobalProxy` |
| **Health** | `healthCheck`, `checkForUpdate`, `openExternalUrl` |
| **Notifications** | `setNotificationsEnabled`, `getNotificationStatus`, `showTestNotification`, `clearNotificationAttention` |
| **Pending Ops** | `getPendingMailOpsSummary`, `listPendingMailOps` |
| **Trusted Senders** | `trustSender`, `listTrustedSenders`, `removeTrustedSender` |
| **User Data** | `listEmailTemplates`, `saveEmailTemplate`, `deleteEmailTemplate`, `getEmailSignature`, `setEmailSignature` |
| **Diagnostics** | `readAppLog` |

## 布局与视图结构

```
Layout
├── TitleBar (自定义标题栏)
├── Sidebar (侧边栏)
├── main (主内容区)
│   ├── OfflineBanner (离线横幅)
│   ├── ViewErrorBoundary (视图错误边界)
│   └── Suspense
│       ├── InboxView (默认)
│       ├── KanbanView
│       ├── SettingsView (lazy)
│       ├── ComposeView (lazy)
│       ├── SearchView (lazy)
│       ├── SnoozedView (lazy)
│       └── StarredView (lazy)
├── ComposeFAB (浮动按钮)
├── StatusBar (状态栏)
├── CommandPalette (命令面板)
└── ToastContainer (Toast)
```

### 懒加载策略
- 设置、撰写、看板、搜索、延迟邮件、星标邮件视图使用 `React.lazy` 延迟加载
- 启动后自动预加载所有懒加载视图

### 全局 Hook 集成
- `useKeyboard`: 键盘快捷键
- `useNetworkStatus`: 网络状态
- `useRealtimePreferenceSync`: 实时同步偏好同步
- `useRealtimeSyncTriggers`: 实时同步触发
- `useNotificationOpenNavigation`: 通知点击导航
- `useCloseToBackground`: 后台运行
- `useTrayI18n`: 托盘菜单国际化
- `useMailtoOpen`: mailto 链接处理

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| React 19 | UI 框架 |
| TypeScript 5.7 | 类型系统 |
| Vite 6 | 构建工具 |
| TailwindCSS 4 | 样式 |
| Zustand 5 | 状态管理 |
| TanStack React Query | 数据获取与缓存 |
| i18next + react-i18next | 国际化 |
| TipTap 3 | 富文本编辑器 (撰写) |
| @dnd-kit | 拖拽排序 |
| lucide-react | 图标 |
| DOMPurify | HTML 消毒 |
| turndown + tiptap-markdown | HTML/Markdown 转换 |
| @tanstack/react-virtual | 虚拟列表 |

## 数据模型

### 视图类型 (activeView)
- `inbox`: 收件箱 (默认)
- `kanban`: 看板
- `compose`: 撰写
- `settings`: 设置
- `search`: 搜索
- `snoozed`: 延迟邮件
- `starred`: 星标邮件

### 主题
- `light`: 亮色
- `dark`: 暗色
- `system`: 跟随系统

## 测试与质量

- Vitest + jsdom 环境
- @testing-library/react
- 命令: `pnpm test`

## 常见问题 (FAQ)

### Q: 前端如何与后端通信？
A: 通过 `@tauri-apps/api` 的 `invoke` 函数调用 Rust 命令，React Query 封装缓存和自动刷新。

### Q: 为什么视图使用懒加载？
A: 减少首屏加载时间，启动后自动预加载保证切换时流畅。

### Q: 国际化支持哪些语言？
A: 通过 i18next 支持多语言，语言文件在 src 中配置。

## 相关文件清单

| 文件 | 说明 |
|------|------|
| `src/main.tsx` | React 入口 |
| `src/App.tsx` | 根组件 + 错误边界 + 闪屏动画 |
| `src/app/Layout.tsx` | 主布局 |
| `src/app/lazyViewPreload.ts` | 懒加载预加载 |
| `src/app/useRealtimePreferenceSync.ts` | 实时同步偏好同步 |
| `src/app/useRealtimeSyncTriggers.ts` | 实时同步触发 |
| `src/app/useNotificationOpenNavigation.ts` | 通知导航 |
| `src/app/useCloseToBackground.ts` | 后台运行 |
| `src/app/useTrayI18n.ts` | 托盘国际化 |
| `src/app/useMailtoOpen.ts` | mailto 处理 |
| `src/features/` | 视图组件 |
| `src/components/` | 通用组件 |
| `src/stores/` | Zustand stores |
| `src/hooks/` | React Hooks |
| `src/lib/` | 工具库 |
| `src/styles/` | 样式 |
| `tsconfig.json` | TypeScript 配置 |
| `vite.config.ts` | Vite 配置 (推断) |
| `package.json` | 前端依赖 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全前端文件清单 (111 个文件)，按目录分类统计
- 补充 `src/lib/` 目录详解 (15 个文件的用途与大小)
- 补充 `src/lib/api.ts` 详细文档 (617 行，24 个 API 模块分类)
- 更新布局架构章节标题
