[根目录](../../CLAUDE.md) > [crates](../) > **pebble-crypto**

# pebble-crypto

## 模块职责

AES-GCM 加密服务。使用操作系统密钥链 (keyring) 存储数据加密密钥 (DEK)，为敏感数据 (如邮件账号密码、OAuth token) 提供加密存储。

## 入口与启动

- **入口文件**: `src/lib.rs`
- **关键 API**:
  - `CryptoService::init()`: 从 OS 密钥链获取或创建 DEK
  - `crypto_service.encrypt(plaintext)`: 加密
  - `crypto_service.decrypt(ciphertext)`: 解密

## 对外接口

### 公开模块
| 模块 | 说明 |
|------|------|
| `aes` | AES-GCM 加密/解密实现 |
| `keystore` | OS 密钥链集成 (KeyStore) |

### KeyStore
- `get_or_create_dek()`: 从密钥链获取 32 字节 DEK，不存在则创建并存储

### CryptoService
- 持有 `Zeroizing<[u8; 32]>` 类型的 DEK (内存安全: 使用后立即清零)

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| aes-gcm | AES-256-GCM 加密 |
| keyring | OS 密钥链 (跨平台: macOS Keychain, Windows Credential Manager, Linux libsecret) |
| rand | 随机数生成 (DEK 创建) |
| zeroize | 安全内存清零 |
| pebble-core | Result 类型 |
| tracing | 日志 |

## 数据模型

### 加密流程
1. 初始化: 从 OS 密钥链读取 DEK，不存在则生成 32 字节随机密钥并存储
2. 加密: AES-256-GCM (随机 nonce)
3. 解密: AES-256-GCM (从密文中提取 nonce)

## 测试与质量

- 内置测试 (标记为 `#[ignore]`):
  - `test_crypto_service_init`: 测试初始化 (需要 OS 密钥链)
  - `test_crypto_service_round_trip`: 测试加解密往返 (需要 OS 密钥链)

## 常见问题 (FAQ)

### Q: 为什么使用 OS 密钥链？
A: 避免将加密密钥硬编码或明文存储在磁盘上，利用操作系统级别的安全存储。

### Q: Zeroizing 是什么？
A: zeroize crate 的类型，确保在 Drop 时安全清零内存中的敏感数据。

## 文件清单 (3 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | CryptoService 主体 |
| `src/aes.rs` | AES-GCM 实现 |
| `src/keystore.rs` | 密钥链管理 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (3 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
