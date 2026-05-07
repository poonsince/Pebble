<p align="center">
  <img src="src/assets/app-icon.png" alt="Pebble logo" width="120">
</p>

<h1 align="center">Pebble</h1>

<p align="center">
  一个以 Ubuntu 22.04 桌面兼容性为主要目标的 Pebble 个人分支。
</p>

<p align="center">
  <a href="README.md">English</a>
  ·
  <a href="LICENSE">许可证</a>
</p>

## 项目说明

本仓库 fork 自 [QingJ01/Pebble](https://github.com/QingJ01/Pebble)。

这个分支主要用于适配 Ubuntu 22.04 桌面环境，重点处理 Linux 桌面运行与打包兼容性问题，主要用途是自用。

Pebble 本身是一个使用 Rust、Tauri 和 React 构建的本地优先桌面邮件客户端。

## 这个分支的重点

- 保持 Pebble 在 Ubuntu 22.04 上可用
- 修复 Linux 打包和运行时问题
- 维护一个可在本地桌面环境使用的自用版本

## 技术栈

- Rust
- Tauri 2
- React 19
- TypeScript
- SQLite

## 环境要求

- Rust stable
- Node.js 18+
- pnpm 8+
- 当前平台所需的 Tauri 系统依赖

在 Ubuntu 22.04 上构建前，需要先安装 Linux 桌面依赖。

## 快速开始

```bash
git clone https://github.com/QingJ01/Pebble.git
cd Pebble

pnpm install
cp .env.example .env
pnpm dev
```

## 常用命令

```bash
pnpm dev
pnpm build:frontend
pnpm build:linux
cargo check
pnpm test
```

## OAuth

如果需要使用 Gmail 或 Outlook OAuth，请将 `.env.example` 复制为 `.env`，并填写你实际使用的提供商凭据。

## 许可证

本项目继续使用 [GNU Affero General Public License v3.0](LICENSE)。
