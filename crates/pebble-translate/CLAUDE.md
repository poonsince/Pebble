[根目录](../../CLAUDE.md) > [crates](../) > **pebble-translate**

# pebble-translate

## 模块职责

多提供商邮件翻译服务。支持 DeepL、DeepLX、自定义 API 和 LLM 翻译引擎，将邮件内容从源语言翻译到目标语言。

## 入口与启动

- **入口文件**: `src/lib.rs`
- **关键 API**:
  - `TranslateService::translate(config, text, from, to)`: 异步翻译

## 对外接口

### 公开模块
| 模块 | 说明 |
|------|------|
| `deepl` | DeepL API 客户端 |
| `deeplx` | DeepLX (开源替代) 客户端 |
| `generic` | 通用 HTTP API 翻译 (可适配任意翻译 API) |
| `llm` | LLM 翻译 (OpenAI 兼容接口) |
| `types` | `TranslateProviderConfig`, `TranslateResult` |

### TranslateProviderConfig (枚举)
- `DeepLX { endpoint }`
- `DeepL { api_key, use_free_api }`
- `GenericApi { endpoint, api_key, source_lang_param, target_lang_param, text_param, result_path }`
- `LLM { endpoint, api_key, model, mode }`

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| reqwest | HTTP 客户端 |
| tokio | 异步运行时 |
| pebble-core | Result 类型 |
| serde + serde_json | JSON 解析 |
| tracing | 日志 |

## 数据模型

### TranslateResult
包含翻译后的文本和元数据

## 测试与质量

- 本 crate 无独立 `#[cfg(test)]` 模块

## 常见问题 (FAQ)

### Q: Free API 是什么？
A: DeepL 提供的免费 API 端点 (api-free.deepl.com)。

### Q: LLM 模式如何使用？
A: 配置 LLM endpoint 和 API key，通过 OpenAI 兼容接口调用翻译。

## 文件清单 (6 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | TranslateService 主体 |
| `src/deepl.rs` | DeepL API |
| `src/deeplx.rs` | DeepLX API |
| `src/generic.rs` | 通用 API |
| `src/llm.rs` | LLM 翻译 |
| `src/types.rs` | 配置和结果类型 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (6 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
