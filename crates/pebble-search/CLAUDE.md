[根目录](../../CLAUDE.md) > [crates](../) > **pebble-search**

# pebble-search

## 模块职责

基于 Tantivy 的全文搜索引擎。负责邮件消息的索引、搜索 (全文 + 高级条件)、schema 管理、增量更新与重建。

## 入口与启动

- **入口文件**: `src/lib.rs`
- **关键 API**:
  - `TantivySearch::open(path)`: 打开索引 (自动检测 schema 变更)
  - `TantivySearch::open_in_memory()`: 内存索引 (测试用)
  - `needs_reindex()`: 是否需要重建索引
  - `doc_count()`: 索引文档数

## 对外接口

### 核心方法
| 方法 | 说明 |
|------|------|
| `index_message(msg, folder_ids)` | 索引单封邮件 |
| `index_messages_batch([(msg, folder_ids)])` | 批量索引 |
| `remove_message(message_id)` | 删除索引项 |
| `delete_by_account(account_id)` | 删除账号所有索引 |
| `search(query, limit)` | 全文搜索 |
| `advanced_search(params)` | 高级搜索 (from/to/subject/date/attachment/folder) |
| `commit()` | 提交索引变更 |
| `clear_index()` | 清空索引 |

### 高级搜索参数 (AdvancedSearchParams)
- `text`: 全文关键词
- `from`: 发件人
- `to`: 收件人
- `subject`: 主题
- `date_from` / `date_to`: 日期范围
- `has_attachment`: 是否有附件
- `folder_id`: 文件夹过滤
- `limit`: 结果数量

### Schema
- 自定义 tokenizer: `BODY_TOKENIZER` (正文分词), `NGRAM_TOKENIZER` (ngram 分词)
- 字段: message_id, subject, body_text, from_address, from_name, to_addresses, date, folder_id, account_id, has_attachment

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| tantivy | 全文搜索引擎 |
| pebble-core | 共享类型 + SearchHit trait |
| tracing | 日志 |

## 数据模型

### 索引 Schema
- **message_id**: STRING + STORED (唯一标识)
- **subject**: NGRAM + STORED (主题搜索)
- **body_text**: BODY_TOKENIZER + STORED (正文搜索)
- **from_address**: NGRAM + STORED (发件人搜索)
- **from_name**: NGRAM + STORED (发件人姓名搜索)
- **to_addresses**: NGRAM (收件人搜索)
- **date**: DATE + STORED (日期范围)
- **folder_id**: STRING (文件夹过滤)
- **account_id**: STRING (账号过滤)
- **has_attachment**: STRING (附件过滤)

## 测试与质量

- 内置 11 个单元测试:
  - 基础搜索 (主题、正文、发件人)
  - 大小写不敏感搜索
  - Schema 变更检测 (旧 tokenizer 触发重建)
  - 无结果搜索
  - 清空索引
  - 重新索引替换旧文档
  - CJK (中文) 搜索
  - Snippet 内容验证
  - 按账号删除
  - CC 收件人搜索

## 常见问题 (FAQ)

### Q: 为什么使用 Tantivy 而不是 SQLite FTS?
A: Tantivy 提供更灵活的分词器、ngram 支持 (对 CJK 语言重要)、更好的搜索相关性。

### Q: 索引重建何时触发？
A: schema 变更 (如 tokenizer 变更)、索引为空但数据库有数据、计数不匹配时。

## 文件清单 (2 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | 搜索引擎主体 |
| `src/schema.rs` | Schema 定义 + tokenizer 注册 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (2 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
- 记录完整 API 和测试覆盖
