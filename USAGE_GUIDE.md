# ltmatrix 使用指南

## 目录

1. [简介](#简介)
2. [安装](#安装)
3. [快速开始](#快速开始)
4. [配置](#配置)
5. [命令行选项](#命令行选项)
6. [执行模式](#执行模式)
7. [子命令](#子命令)
8. [示例](#示例)
9. [工作流程](#工作流程)
10. [故障排除](#故障排除)

---

## 简介

ltmatrix 是一个高性能、跨平台的长时任务代理编排器（Long-Time Agent Orchestrator）。它使用 AI 代理（如 Claude）自动化软件开发任务，将复杂目标分解为可执行的任务，并通过完整的测试和验证流程执行这些任务。

### 主要特性

- 🤖 **多代理支持**: 支持 Claude、OpenCode、KimiCode、Codex 等多种 AI 代理
- 🔄 **6 阶段流水线**: 生成 → 评估 → 执行 → 测试 → 验证 → 提交
- ⚡ **三种执行模式**: 快速模式、标准模式、专家模式
- 🔧 **灵活配置**: 支持全局配置和项目级配置
- 📊 **任务依赖管理**: 自动处理任务间的依赖关系
- 💾 **会话持久化**: 支持中断后恢复工作

---

## 安装

### 从源码构建

```bash
# 克隆仓库
git clone https://github.com/bigfish1913/lt_matrix.git
cd lt_matrix

# 构建
cargo build --release

# 二进制文件位于
./target/release/ltmatrix
```

### 平台支持

| 平台 | 架构 | 状态 |
|------|------|------|
| Windows | x86_64 | ✅ 生产就绪 |
| Linux | x86_64 | ✅ 生产就绪 |
| Windows | ARM64 | ⏳ 需要工具链 |
| Linux | ARM64 | ⏳ 需要工具链 |

---

## 快速开始

### 1. 配置 AI 代理

创建配置文件 `~/.ltmatrix/config.toml`:

```toml
default = "claude"

[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
```

### 2. 运行你的第一个任务

```bash
# 标准模式
ltmatrix "创建一个简单的 Hello World 程序"

# 快速模式（快速迭代）
ltmatrix --fast "添加错误处理"

# 专家模式（最高质量）
ltmatrix --expert "实现用户认证系统"
```

---

## 配置

### 配置文件位置

ltmatrix 按以下顺序查找配置文件（后面的覆盖前面的）：

1. 全局配置: `~/.ltmatrix/config.toml` (Windows: `C:\Users\<用户名>\.ltmatrix\config.toml`)
2. 项目配置: `<项目目录>/.ltmatrix/config.toml`
3. 命令行参数

### 配置文件示例

```toml
# 默认代理
default = "claude"

# 代理配置
[agents.claude]
command = "claude"
model = "claude-sonnet-4-6"
timeout = 3600
api_key = "your-api-key"     # 可选，也可使用环境变量
base_url = "https://api.anthropic.com"  # 可选，自定义 API 端点

[agents.opencode]
command = "opencode"
model = "gpt-4"
timeout = 1800

# 模式预设
[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true
max_retries = 1
max_depth = 2
timeout_plan = 60
timeout_exec = 1800

[modes.standard]
model = "claude-sonnet-4-6"
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120
timeout_exec = 3600

[modes.expert]
model = "claude-opus-4-6"
run_tests = true
verify = true
max_retries = 5
max_depth = 5
timeout_plan = 300
timeout_exec = 7200

# 输出配置
[output]
format = "text"    # text 或 json
colored = true
progress = true

# 日志配置
[logging]
level = "info"     # trace, debug, info, warn, error
```

### 环境变量

```bash
# API 密钥（推荐使用配置文件或环境变量）
export ANTHROPIC_API_KEY="your-api-key"

# 禁用彩色输出
export NO_COLOR=1
```

---

## 命令行选项

### 基本用法

```bash
ltmatrix [选项] <目标>
```

### 全局选项

| 选项 | 说明 |
|------|------|
| `--no-color` | 禁用彩色输出 |

### 输入选项

| 选项 | 简写 | 说明 |
|------|------|------|
| `<goal>` | - | 要完成的目标/任务 |
| `--file <FILE>` | `-f` | 从文件读取目标 |

### 执行模式

| 选项 | 说明 |
|------|------|
| `--fast` | 快速执行模式 |
| `--expert` | 专家执行模式 |
| `--mode <MODE>` | 指定执行模式 (fast/standard/expert) |

### 配置选项

| 选项 | 简写 | 说明 |
|------|------|------|
| `--config <FILE>` | `-c` | 配置文件路径 |
| `--agent <AGENT>` | - | 使用的代理后端 |

### 输出选项

| 选项 | 说明 |
|------|------|
| `--output <FORMAT>` | 输出格式 (text/json/json-compact) |
| `--log-level <LEVEL>` | 日志级别 (trace/debug/info/warn/error) |
| `--log-file <FILE>` | 日志文件路径 |

### 执行控制

| 选项 | 说明 |
|------|------|
| `--max-retries <NUM>` | 每个任务的最大重试次数 |
| `--timeout <SECONDS>` | 操作超时时间（秒） |
| `--dry-run` | 生成计划但不执行 |
| `--resume` | 恢复中断的工作 |
| `--ask` | 在规划前请求澄清 |
| `--regenerate-plan` | 重新生成计划 |
| `--on-blocked <STRATEGY>` | 阻塞任务处理策略 (skip/ask/abort/retry) |

### 其他选项

| 选项 | 说明 |
|------|------|
| `--mcp-config <FILE>` | MCP 配置文件 |
| `--telemetry` | 启用匿名使用遥测 |

---

## 执行模式

### 快速模式 (`--fast`)

- **用途**: 快速迭代、简单任务
- **模型**: claude-haiku-4-5
- **特点**:
  - 跳过测试
  - 最少重试 (1次)
  - 短超时
  - 适合原型开发

```bash
ltmatrix --fast "添加一个简单的日志函数"
```

### 标准模式 (默认)

- **用途**: 日常开发任务
- **模型**: claude-sonnet-4-6
- **特点**:
  - 运行测试
  - 适度重试 (3次)
  - 平衡速度和质量

```bash
ltmatrix "实现用户注册功能"
```

### 专家模式 (`--expert`)

- **用途**: 复杂任务、关键功能
- **模型**: claude-opus-4-6
- **特点**:
  - 完整测试和验证
  - 最多重试 (5次)
  - 最长超时
  - 最高质量输出

```bash
ltmatrix --expert "设计并实现微服务架构"
```

---

## 子命令

### completions - 生成 Shell 补全

```bash
# 生成 Bash 补全
ltmatrix completions bash

# 生成 Zsh 补全
ltmatrix completions zsh

# 生成 Fish 补全
ltmatrix completions fish

# 生成 PowerShell 补全
ltmatrix completions powershell

# 显示安装说明
ltmatrix completions bash --install
```

### man - 生成手册页

```bash
# 生成到默认目录 (./man)
ltmatrix man

# 指定输出目录
ltmatrix man --output /usr/local/share/man
```

### release - 创建发布构建

```bash
# 构建当前平台
ltmatrix release

# 指定目标平台
ltmatrix release --target x86_64-unknown-linux-musl

# 创建归档文件
ltmatrix release --archive

# 构建所有支持的平台
ltmatrix release --all-targets
```

### cleanup - 清理工作区状态

```bash
# 重置所有任务为待处理状态
ltmatrix cleanup --reset-all

# 只重置失败的任务
ltmatrix cleanup --reset-failed

# 删除所有工作区状态文件
ltmatrix cleanup --remove

# 强制清理（无需确认）
ltmatrix cleanup --remove --force

# 预览模式（显示将要清理的内容）
ltmatrix cleanup --dry-run
```

### memory - 管理项目记忆

```bash
# 显示记忆状态
ltmatrix memory status

# 显示 JSON 格式的状态
ltmatrix memory status --json

# 压缩记忆条目
ltmatrix memory summarize

# 强制压缩
ltmatrix memory summarize --force

# 预览压缩操作
ltmatrix memory summarize --dry-run

# 清除所有记忆
ltmatrix memory clear

# 强制清除（无需确认）
ltmatrix memory clear --force
```

---

## 示例

### 基本示例

```bash
# 创建一个简单的 REST API
ltmatrix "创建一个用户管理 REST API，包含 CRUD 操作"

# 从文件读取目标
ltmatrix --file requirements.txt

# 使用特定代理
ltmatrix --agent opencode "添加单元测试"
```

### 快速原型

```bash
# 快速添加功能
ltmatrix --fast "添加一个健康检查端点"

# 快速修复 bug
ltmatrix --fast "修复登录页面的样式问题"
```

### 复杂项目

```bash
# 使用专家模式处理复杂任务
ltmatrix --expert "实现完整的订单处理系统，包含：
- 订单创建和验证
- 库存检查
- 支付集成
- 邮件通知
- 订单追踪"

# 预览计划
ltmatrix --dry-run "重构数据库访问层"
```

### 恢复和调试

```bash
# 恢复中断的工作
ltmatrix --resume

# 重新生成计划
ltmatrix --regenerate-plan "优化查询性能"

# 详细日志
ltmatrix --log-level debug "实现缓存机制"
```

### 输出格式

```bash
# JSON 输出（便于脚本处理）
ltmatrix --output json "生成 API 文档"

# 保存日志到文件
ltmatrix --log-file debug.log "实现搜索功能"
```

---

## 工作流程

### 6 阶段流水线

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Generate  │ ──▶ │    Assess   │ ──▶ │   Execute   │
│   生成任务   │     │   评估任务   │     │   执行任务   │
└─────────────┘     └─────────────┘     └─────────────┘
                                              │
┌─────────────┐     ┌─────────────┐     ┌─────▼───────┐
│   Commit    │ ◀── │   Verify    │ ◀── │    Test     │
│   提交更改   │     │   验证结果   │     │   运行测试   │
└─────────────┘     └─────────────┘     └─────────────┘
```

### 阶段说明

1. **Generate (生成)**: 分析目标，生成任务列表
2. **Assess (评估)**: 评估任务复杂度和依赖关系
3. **Execute (执行)**: 按依赖顺序执行任务
4. **Test (测试)**: 运行测试验证功能
5. **Verify (验证)**: 验证代码质量和规范
6. **Commit (提交)**: 将更改提交到 Git

### 任务依赖

ltmatrix 自动管理任务依赖：

- 识别任务间的依赖关系
- 按拓扑顺序执行任务
- 并行执行独立的任务
- 处理循环依赖

---

## 故障排除

### 常见问题

#### 1. 代理未找到

```
错误: Unsupported agent backend 'xxx'
```

**解决方案**: 检查配置文件中的代理名称，确保使用支持的代理（claude、opencode、kimicode、codex）。

#### 2. API 密钥问题

```
错误: ANTHROPIC_API_KEY not set
```

**解决方案**:
```bash
# 方法1: 设置环境变量
export ANTHROPIC_API_KEY="your-api-key"

# 方法2: 在配置文件中设置
[agents.claude]
api_key = "your-api-key"
```

#### 3. 嵌套执行错误

```
错误: Claude Code cannot be launched inside another Claude Code session
```

**解决方案**: ltmatrix 会自动处理此问题，但如果仍然出现，请确保不在另一个 Claude Code 会话中运行。

#### 4. 配置文件未找到

**解决方案**: 确保配置文件位于正确位置：
- Windows: `C:\Users\<用户名>\.ltmatrix\config.toml`
- Linux/macOS: `~/.ltmatrix/config.toml`

#### 5. 权限错误

**解决方案**: 确保对项目目录有读写权限。

### 调试技巧

```bash
# 启用详细日志
ltmatrix --log-level debug "your goal"

# 保存日志到文件
ltmatrix --log-file debug.log "your goal"

# 使用 dry-run 预览计划
ltmatrix --dry-run "your goal"
```

### 获取帮助

```bash
# 显示帮助
ltmatrix --help

# 显示版本
ltmatrix --version

# 查看子命令帮助
ltmatrix completions --help
```

---

## 附录

### 支持的代理后端

| 代理 | 命令 | 说明 |
|------|------|------|
| Claude | `claude` | Anthropic Claude CLI |
| OpenCode | `opencode` | OpenCode CLI |
| KimiCode | `kimicode` | KimiCode CLI |
| Codex | `codex` | OpenAI Codex |

### 配置优先级

1. 命令行参数（最高优先级）
2. 项目配置 (`.ltmatrix/config.toml`)
3. 全局配置 (`~/.ltmatrix/config.toml`)
4. 默认值（最低优先级）

### 项目结构

```
project/
├── .ltmatrix/
│   ├── config.toml        # 项目配置
│   ├── sessions/          # 会话数据
│   └── tasks-manifest.json # 任务状态
├── logs/                  # 运行日志
│   └── run-*.log
└── ... (项目文件)
```

---

## 许可证

MIT License - 详见 [LICENSE](LICENSE) 文件。

## 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

## 支持

- GitHub Issues: https://github.com/bigfish1913/lt_matrix/issues
- 文档: https://github.com/bigfish1913/lt_matrix#readme
