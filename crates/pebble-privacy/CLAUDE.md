[根目录](../../CLAUDE.md) > [crates](../) > **pebble-privacy**

# pebble-privacy

## 模块职责

邮件隐私保护。HTML 内容消毒 (sanitization) 和邮件追踪器拦截 (tracker blocking)，防止发件人追踪收件人行为。

## 入口与启动

- **入口文件**: `src/lib.rs`
- **公开导出**: `PrivacyGuard`

## 对外接口

### 公开模块
| 模块 | 说明 |
|------|------|
| `sanitizer` | HTML 消毒处理 |
| `tracker` | 邮件追踪器检测与拦截 |

### PrivacyGuard
主要的隐私保护接口，提供:
- HTML 内容消毒 (去除危险标签和属性)
- 追踪像素/图片拦截
- 远程资源引用清理

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| ammonia | HTML 消毒 (基于 HTML5ever) |
| lol_html | 高性能流式 HTML 重写 |
| pebble-core | 错误类型 |

## 数据模型

### 消毒策略
- 去除 `<script>` 等危险标签
- 剥离事件处理器 (onclick, onerror 等)
- 拦截远程图片加载 (追踪像素)
- 可选: 重写远程资源 URL

## 测试与质量

- 本 crate 无独立 `#[cfg(test)]` 模块

## 常见问题 (FAQ)

### Q: 为什么同时使用 ammonia 和 lol_html？
A: ammonia 提供安全白名单消毒，lol_html 提供高性能流式 HTML 重写，两者互补。

## 文件清单 (3 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | PrivacyGuard 入口 |
| `src/sanitizer.rs` | HTML 消毒 (ammonia + lol_html) |
| `src/tracker.rs` | 追踪器检测与拦截 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (3 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
