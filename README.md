<p align="center">
  <img src="src/assets/app-icon.png" alt="Pebble logo" width="120">
</p>

<h1 align="center">Pebble</h1>

<p align="center">
  A personal fork of Pebble focused on Ubuntu 22.04 desktop compatibility.
</p>

<p align="center">
  <a href="README.zh-CN.md">简体中文</a>
  ·
  <a href="LICENSE">License</a>
</p>

## Overview

This repository is forked from [QingJ01/Pebble](https://github.com/QingJ01/Pebble).

It is mainly used to adapt Pebble to the Ubuntu 22.04 desktop environment, especially around Linux desktop packaging and runtime compatibility. The primary purpose of this fork is personal use.

Pebble itself is a local-first desktop mail client built with Rust, Tauri, and React.

## Scope Of This Fork

- Keep Pebble usable on Ubuntu 22.04
- Fix Linux packaging and runtime issues
- Maintain a practical local desktop build for personal use

## Tech Stack

- Rust
- Tauri 2
- React 19
- TypeScript
- SQLite

## Requirements

- Rust stable
- Node.js 18+
- pnpm 8+
- Tauri system dependencies for your platform

For Ubuntu 22.04, Linux desktop dependencies are required before building.

## Quick Start

```bash
git clone https://github.com/QingJ01/Pebble.git
cd Pebble

pnpm install
cp .env.example .env
pnpm dev
```

## Common Commands

```bash
pnpm dev
pnpm build:frontend
pnpm build:linux
cargo check
pnpm test
```

## OAuth

If you need Gmail or Outlook OAuth, copy `.env.example` to `.env` and fill in the provider credentials you use.

## License

This project remains under the [GNU Affero General Public License v3.0](LICENSE).
