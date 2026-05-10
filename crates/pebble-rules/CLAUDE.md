[根目录](../../CLAUDE.md) > [crates](../) > **pebble-rules**

# pebble-rules

## 模块职责

自动化规则引擎。根据预定义条件匹配邮件消息，并执行相应动作 (添加标签、标记已读等)。

## 入口与启动

- **入口文件**: `src/lib.rs`
- **关键 API**:
  - `RuleEngine::new(rules)`: 从数据库规则构建引擎
  - `rule_engine.evaluate(message)`: 评估消息，返回匹配的动作列表
  - `rule_engine.rule_count()`: 活跃规则数量

## 对外接口

### 公开模块
| 模块 | 说明 |
|------|------|
| `matcher` | 条件评估 (`evaluate_conditions`) |
| `types` | `RuleConditionSet`, `RuleAction` |

### RuleEngine
- 过滤已禁用的规则
- 解析 JSON 条件/动作 (失败则跳过)
- 按 priority 排序
- `evaluate()` 对消息依次评估所有活跃规则

## 关键依赖与配置

| 依赖 | 用途 |
|------|------|
| serde + serde_json | JSON 解析 |
| pebble-core | Rule 类型、Message 类型 |
| tracing | 日志 (跳过无效规则时警告) |

## 数据模型

### RuleConditionSet
- `operator`: "and" / "or"
- `conditions`: 条件数组
  - `field`: "from", "subject", "to", "body" 等
  - `op`: "contains", "equals", "starts_with" 等
  - `value`: 匹配值

### RuleAction (枚举)
- `AddLabel(value)`: 添加标签
- `MarkRead`: 标记已读
- 其他动作类型...

## 测试与质量

- 内置单元测试:
  - `test_rule_engine_evaluate`: 验证规则匹配 (条件: from contains "newsletter", 动作: AddLabel + MarkRead)
  - `test_disabled_rules_skipped`: 验证禁用规则被跳过

## 常见问题 (FAQ)

### Q: 规则如何存储？
A: 存储在 SQLite (pebble-store/rules)，条件和动作为 JSON 字符串，RuleEngine 构造时解析。

### Q: 规则何时执行？
A: 在邮件同步过程中，新邮件入库后触发规则评估。

## 文件清单 (3 个文件)

| 文件 | 说明 |
|------|------|
| `src/lib.rs` | RuleEngine 主体 |
| `src/matcher.rs` | 条件评估逻辑 |
| `src/types.rs` | 条件集和动作类型定义 |
| `Cargo.toml` | crate 配置 |

## 变更记录 (Changelog)

### 2026-05-06 22:21:00
- 补全 crate 子模块文件清单 (3 个文件)

### 2026-05-06 22:10:36
- 初始模块文档生成
