# ltmatrix - Rust 重写需求文档

## 项目概述

将 Python 版本的 `longtime.py` 重写为 Rust，打造一个高性能、跨平台的长期运行 Agent 编排器。

**项目名称**: ltmatrix (Long-Time Agent Network)

---

## Python 基线现状

> 实现 Rust 版本前，先了解 Python 参考实现的当前状态。

### 已有功能（需在 Rust 中保留）

| 功能 | 说明 |
|------|------|
| 6 阶段流水线 | Generate → Assess → Execute → Test → Verify → Commit |
| 任务依赖调度 | 基于 `depends_on` 的拓扑执行，支持并行 |
| 任务复杂度评估 | Claude 评估后可拆分为子任务（最深 3 层） |
| Session 复用 | AgentPool：retry 复用自身 session，依赖链传递 session |
| 自动测试 | 检测 pytest/npm/go/cargo 并运行，失败后自动尝试修复 |
| MCP e2e 测试 | 通过 `--mcp-config` 注入 Playwright 等工具 |
| Git 集成 | 自动 init、`.gitignore` 生成、按任务独立 commit |
| 项目记忆 | `.claude/memory.md` 跨任务传递架构决策 |
| 进度与拓扑图 | ASCII 拓扑视图 + Mermaid 图保存 |
| Prompt 截断 | 防止超过 ARG_MAX，分优先级裁剪 |
| 依赖方案验证 | 生成后检测缺失依赖和循环依赖 |
| 工作区恢复 | `--resume` 继续中断的任务，自动重置 in_progress |
| 交互式澄清 | `--ask` 生成规划前向用户提问 |

### 已知缺陷（已在基线中修复）

| Bug | 修复内容 |
|-----|---------|
| 模型名错误 | `MODEL_FAST/SMART` 原为 `"glm-5"`，已改为 `claude-sonnet-4-6` / `claude-opus-4-6` |
| 状态值拼写错误 | resume 检测时 `t.status == "done"` 应为 `"completed"`，导致 done 统计永远为 0 |
| 中文硬编码提示 | resume 交互提示含中文，已改为英文 |
| 无循环依赖检测 | 现已在 `generate_tasks` 后调用 `validate_dependencies()` |

### Python 未实现（Rust 需新增）

- 多 Agent 后端（仅支持 `claude`）
- 执行模式（`--fast` / `--expert`）
- TOML 配置文件
- JSON 输出格式（`--output json`）
- 结构化日志级别（TRACE/DEBUG/INFO/WARN/ERROR）
- `--dry-run` 只生成任务不执行
- `--on-blocked` 阻塞任务处理策略
- `--regenerate-plan` 重新生成任务方案
- `release` 子命令
- 实时进度条（目前为纯文本打印）
- Git branch + squash merge（~~Python 直接 commit 到当前分支，Rust 版本需实现 per-task branch 后 squash merge~~已在 Python 基线实现）

---

## 核心需求

### 1. 多 Agent 后端支持

支持多种代码 Agent 后端，通过统一接口调用： 

| Agent | CLI 命令 | 说明 |
|-------|---------|------|
| Claude | `claude` | 默认后端，使用 Claude Code CLI |
| OpenCode | `opencode` | 开源代码助手 |
| KimiCode | `kimi-code` | 月之暗面代码助手 |
| Codex | `codex` | OpenAI Codex |

**配置方式：**
```bash
# 命令行指定
ltmatrix --agent claude "build a REST API"

# 配置文件 ~/.ltmatrix/config.toml
[default]
agent = "claude"

[agents.claude]
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4"
```

### 2. 跨平台二进制发布

**目标平台：**
- Windows (x86_64, ARM64)
- Linux (x86_64, ARM64)
- macOS (Intel, Apple Silicon)

**安装方式：**
```bash
# Homebrew (macOS/Linux)
brew install ltmatrix

# Cargo
cargo install ltmatrix

# Scoop (Windows)
scoop install ltmatrix

# 直接下载二进制
# GitHub Releases 提供 statically-linked 二进制
```

**技术要求：**
- 使用 `cross` 或 `cargo-zigbuild` 进行交叉编译
- 静态链接减少依赖 (musl for Linux)
- 单文件二进制，无运行时依赖

### 3. 丰富的输出与日志系统

**日志级别：**
```
TRACE  - 每个 Claude 调用的完整 prompt/response
DEBUG  - 任务调度细节、文件变更
INFO   - 任务开始/完成、进度摘要 (默认)
WARN   - 重试、跳过的任务
ERROR  - 失败详情
```

**输出格式：**
```bash
# 终端美化输出 (默认)
ltmatrix "goal"

# JSON 格式 (便于解析)
ltmatrix "goal" --output json

# 结构化日志 (便于调试)
ltmatrix "goal" --log-level debug --log-file run.log
```

**进度展示：**
- 实时进度条 (当前任务/总任务)
- ETA 估算
- 任务状态树形视图
- 并行任务实时状态

**报告生成：**
```
logs/
├── run-20260303-143022.log      # 完整日志
├── tasks-manifest.json          # 任务清单
└── final-report.md              # 最终报告
```

### 4. 三种执行模式

| 模式 | 说明 | 特点 |
|------|------|------|
| **快速模式** `--fast` | 最快完成 | 使用 Haiku/Sonnet，跳过测试，最小验证 |
| **标准模式** (默认) | 与 longtime.py 逻辑一致 | 完整 6 阶段流水线 |
| **专家模式** `--expert` | 最高质量 | Opus 执行，完整测试套件，代码审查 |

**标准模式流水线 (与 longtime.py 保持一致)：**
```
┌─────────────────────────────────────────────────────────────────┐
│  标准模式 (Standard Mode) - 默认                                  │
├─────────────────────────────────────────────────────────────────┤
│  1. Generate  → Claude 将目标拆解为任务列表 (JSON)                │
│               └─ 验证依赖完整性（缺失引用 / 循环依赖检测）          │
│  2. Assess    → 评估任务复杂度，必要时拆分为子任务 (max depth=3)    │
│  3. Execute   → Claude 实现任务 (Sonnet 主力，复杂任务用 Opus)     │
│  4. Test      → 编写并运行单元测试 (自动检测: pytest/npm test/...)  │
│               └─ 失败时先尝试 fix_test_failure，再考虑 retry       │
│  5. Verify    → Claude 审查任务完成度                             │
│  6. Commit    → Git 提交（per-task branch + squash merge 到主分支）     │
│  7. Memory    → 提取关键决策写入 .claude/memory.md               │
│                                                                 │
│  配置: model=claude-sonnet-4-6, max_retries=3, timeout=3600s    │
└─────────────────────────────────────────────────────────────────┘
```

**快速模式流水线：**
```
Generate → Assess → Execute → Verify → Commit
                    ↑
              跳过 Test 阶段，使用 Haiku 加速
```

**专家模式额外流程：**
```
Execute → Test → Code Review → Fix Issues → Verify → Commit
                        ↑
                   独立 Review Agent
                   检查: 代码质量、安全、性能、最佳实践
```

**配置示例：**
```toml
[modes.fast]
model = "claude-haiku-4-5"
run_tests = false
verify = true          # 保留验证
max_retries = 1
max_depth = 2          # 减少拆分深度

[modes.standard]
# 与 longtime.py 完全一致
model_fast = "claude-sonnet-4-6"    # 简单任务
model_smart = "claude-opus-4-6"     # 复杂任务 (复杂度评估后)
run_tests = true
verify = true
max_retries = 3
max_depth = 3
timeout_plan = 120                  # 规划阶段超时
timeout_exec = 3600                 # 执行阶段超时

[modes.expert]
model = "claude-opus-4-6"           # 全部使用 Opus
run_tests = true
verify = true
code_review = true                  # 新增代码审查
max_retries = 5
```

### 5. 开源与部署友好

**项目结构：**
```
ltmatrix/
├── Cargo.toml
├── README.md              # 英文主文档
├── README_CN.md           # 中文文档
├── LICENSE                # MIT / Apache-2.0
├── CONTRIBUTING.md        # 贡献指南
├── CHANGELOG.md           # 版本变更
├── .github/
│   ├── workflows/
│   │   ├── ci.yml         # CI: 测试、lint、构建
│   │   └── release.yml    # 自动发布二进制
│   ├── ISSUE_TEMPLATE/
│   └── PULL_REQUEST_TEMPLATE.md
├── src/
│   ├── main.rs
│   ├── agent/             # Agent 后端抽象
│   ├── task/              # 任务管理
│   ├── orchestrator/      # 编排器
│   ├── config/            # 配置处理
│   └── output/            # 输出格式化
├── tests/                 # 集成测试
└── docs/
    ├── ARCHITECTURE.md    # 架构设计
    ├── USAGE.md           # 详细用法
    └── examples/          # 示例配置
```

**CI/CD 要求：**
- PR 检查: `cargo fmt`, `cargo clippy`, `cargo test`
- 自动发布: GitHub Releases 推送二进制
- Docker 镜像: `ghcr.io/user/ltmatrix:latest`

**文档要求：**
- README 包含: 安装、快速开始、配置、示例
- 内置帮助完善: `ltmatrix --help`, `ltmatrix agent --help`
- 配置示例: 提供常见场景的配置模板

---

## 扩展需求

### 6. 强大的 CLI 参数

```bash
ltmatrix [OPTIONS] <GOAL> [PATH]

Arguments:
  <GOAL>    项目目标描述
  [PATH]    工作目录 (默认: ./workspace)

Options:
  -a, --agent <AGENT>          Agent 后端 [default: claude]
  -m, --mode <MODE>            执行模式: fast, standard, expert
  -n, --parallel <N>           并行任务数 [default: 1]
  -d, --doc <FILE>             需求文档路径
  -c, --config <FILE>          配置文件路径
      --mcp-config <FILE>      MCP 配置 (e2e 测试)

Output:
  -o, --output <FORMAT>        输出格式: pretty, json [default: pretty]
  -l, --log-level <LEVEL>      日志级别 [default: info]
      --log-file <FILE>        日志文件路径
  -v, --verbose                详细输出 (等同于 --log-level debug)

Execution:
      --resume                 恢复上次运行
      --dry-run                只生成任务不执行
      --max-depth <N>          任务拆分最大深度 [default: 3]
      --timeout <SECONDS>      单任务超时 [default: 3600]

Models:
      --model <MODEL>          覆盖默认模型
      --model-fast <MODEL>     快速任务模型
      --model-smart <MODEL>    复杂任务模型
```

### 7. 配置文件支持

**配置文件位置 (优先级):**
1. `--config` 指定的路径
2. `./.ltmatrix/config.toml`
3. `~/.ltmatrix/config.toml`

**完整配置示例:**
```toml
# ~/.ltmatrix/config.toml

[general]
default_agent = "claude"
default_mode = "standard"
max_parallel = 3

[logging]
level = "info"
file = "~/.ltmatrix/logs/latest.log"
format = "pretty"  # pretty, json

[agents.claude]
command = "claude"
model_fast = "claude-sonnet-4-6"
model_smart = "claude-opus-4-6"
timeout_plan = 120
timeout_exec = 3600

[agents.opencode]
command = "opencode"
model = "gpt-4-turbo"
timeout_exec = 1800

[execution]
max_retries = 3
max_depth = 3
auto_commit = true
auto_install_deps = true

[git]
user_name = "Longtime Agent"
user_email = "ltmatrix@agent.local"
squash_merge = true
```

### 8. 状态持久化与恢复

**任务状态文件:**
```json
// tasks/manifest.json
{
  "version": "1.0",
  "goal": "Build a REST API",
  "created_at": "2026-03-03T14:30:22Z",
  "updated_at": "2026-03-03T15:45:10Z",
  "status": "in_progress",
  "stats": {
    "total": 12,
    "completed": 8,
    "pending": 3,
    "failed": 1
  }
}

// tasks/task-001.json
{
  "id": "task-001",
  "title": "Setup project structure",
  "status": "completed",
  "started_at": "2026-03-03T14:30:25Z",
  "completed_at": "2026-03-03T14:32:10Z",
  "result": "Created src/, tests/, Cargo.toml",
  "session_id": "sess_abc123"
}
```

**恢复机制:**
- 自动检测中断的任务
- `--resume` 继续上次运行
- 支持从任意任务点恢复

### 9. 任务方案处理

任务方案（Task Plan）是指由 Agent 生成的任务列表及其依赖关系。正确处理任务方案是确保编排器顺利运行的关键。

**任务依赖验证：**

在任务生成阶段完成后，需要验证任务方案的完整性：

```
┌─────────────────────────────────────────────────────────────────┐
│  任务方案验证流程                                                 │
├─────────────────────────────────────────────────────────────────┤
│  1. 依赖存在性检查                                                │
│     ├─ 每个 depends_on 中的 task_id 必须存在                      │
│     └─ 检测: "任务 A 依赖不存在的任务 X"                           │
│                                                                 │
│  2. 循环依赖检测                                                  │
│     ├─ 拓扑排序检测环                                             │
│     └─ 报告: "任务 A → B → C → A 形成循环依赖"                    │
│                                                                 │
│  3. 孤立任务检测                                                  │
│     ├─ 依赖不可达的任务                                           │
│     └─ 警告: "任务 D 依赖已失败/跳过的任务"                        │
│                                                                 │
│  4. 自动修复 (可选)                                               │
│     ├─ 移除无效依赖                                               │
│     ├─ 拆分阻塞任务                                               │
│     └─ 重新生成任务列表                                           │
└─────────────────────────────────────────────────────────────────┘
```

**阻塞任务处理策略：**

当任务因依赖无法满足而被阻塞时，提供多种处理选项：

```bash
# 默认: 警告并跳过阻塞任务
ltmatrix "goal" --on-blocked warn

# 尝试自动修复依赖关系
ltmatrix "goal" --on-blocked auto-fix

# 忽略依赖，强制执行
ltmatrix "goal" --on-blocked force

# 遇到阻塞立即停止
ltmatrix "goal" --on-blocked fail
```

**配置文件设置：**
```toml
[task_plan]
# 依赖验证级别: strict, normal, loose
validation = "normal"

# 阻塞任务处理: warn, auto-fix, force, fail
on_blocked = "warn"

# 是否自动重新生成有问题的任务方案
auto_regenerate = true

# 最大重新生成次数
max_regenerate = 2

# 是否记录任务方案诊断信息
diagnostic_log = true
```

**任务方案诊断输出：**
```
⚠  Task Plan Diagnostic Report
═══════════════════════════════════════════════════════════════

Total tasks: 50
├─ Completed: 24 ✓
├─ Pending: 19
├─ Skipped: 7 (split into subtasks)
└─ Failed: 0

Blocked tasks (19):
├─ task-025 "Implement API endpoints"
│   └─ blocked by: task-024 (status: pending)
├─ task-030 "Add authentication"
│   └─ blocked by: task-029 (not found in plan)
└─ ...

Dependency issues:
├─ Missing dependencies: 1
│   └─ task-030 depends on task-029 (not found)
└─ Circular dependencies: 0

Suggested actions:
1. Run with --on-blocked auto-fix to regenerate plan
2. Manually remove invalid dependencies from task-030
3. Check if task-029 was accidentally skipped during generation
```

**任务方案重新生成：**

当检测到无法解决的依赖问题时，可以触发重新生成：

```
ltmatrix "goal" --regenerate-plan
        │
        ▼
┌───────────────────────────────────────────────────────┐
│  1. 保留已完成任务                                      │
│     └─ 已 completed/skipped 的任务保持不变              │
├───────────────────────────────────────────────────────┤
│  2. 清理阻塞任务                                        │
│     ├─ 删除 pending 状态的任务文件                      │
│     └─ 保留 failed 任务 (可能需要手动处理)              │
├───────────────────────────────────────────────────────┤
│  3. 重新生成任务                                        │
│     ├─ 提供已完成任务的上下文                           │
│     ├─ Agent 基于剩余目标生成新任务                     │
│     └─ 验证新任务方案的完整性                           │
├───────────────────────────────────────────────────────┤
│  4. 继续执行                                            │
│     └─ 从新生成的任务开始执行                           │
└───────────────────────────────────────────────────────┘
```

**API 接口 (Rust)：**
```rust
/// 任务方案验证结果
pub struct PlanValidation {
    pub is_valid: bool,
    pub missing_dependencies: Vec<(String, String)>,  // (task_id, missing_dep)
    pub circular_dependencies: Vec<Vec<String>>,
    pub blocked_tasks: Vec<BlockedTask>,
    pub suggestions: Vec<String>,
}

/// 阻塞任务信息
pub struct BlockedTask {
    pub task_id: String,
    pub title: String,
    pub blocked_by: Vec<Blocker>,
}

pub enum Blocker {
    PendingTask(String),
    MissingTask(String),
    FailedTask(String),
    SkippedTask(String),
}

/// 阻塞处理策略
pub enum OnBlocked {
    Warn,       // 警告并跳过
    AutoFix,    // 尝试自动修复
    Force,      // 忽略依赖强制执行
    Fail,       // 立即失败
}

impl Orchestrator {
    /// 验证任务方案
    pub fn validate_plan(&self) -> PlanValidation;

    /// 重新生成任务方案
    pub fn regenerate_plan(&mut self, keep_completed: bool) -> Result<()>;

    /// 获取可执行的任务 (依赖已满足)
    pub fn get_executable_tasks(&self) -> Vec<&Task>;

    /// 获取阻塞的任务及其原因
    pub fn get_blocked_tasks(&self) -> Vec<BlockedTask>;
}
```

### 10. 快速发布流程 (task-file)

使用任务文件驱动版本发布，实现一键推送和发布新版本。

**发布命令：**
```bash
# 快速发布补丁版本 (0.1.0 → 0.1.1)
ltmatrix release patch

# 发布次版本 (0.1.0 → 0.2.0)
ltmatrix release minor

# 发布主版本 (0.1.0 → 1.0.0)
ltmatrix release major

# 带变更说明发布
ltmatrix release patch --notes "修复了任务恢复的 bug"
```

**发布任务文件 (release-task.md)：**
```markdown
# Release Task

## 版本信息
- 当前版本: {{current_version}}
- 目标版本: {{target_version}}
- 发布类型: {{release_type}} (patch/minor/major)

## 发布清单

### 1. 代码准备
- [ ] 运行所有测试: `cargo test --all`
- [ ] 代码格式化: `cargo fmt --check`
- [ ] 静态检查: `cargo clippy -- -D warnings`
- [ ] 更新 CHANGELOG.md

### 2. 版本更新
- [ ] 更新 Cargo.toml 版本号
- [ ] 更新 Cargo.lock: `cargo update -w`
- [ ] 更新文档中的版本引用

### 3. Git 操作
- [ ] 创建提交: `git commit -m "chore: release v{{target_version}}"`
- [ ] 创建标签: `git tag -a v{{target_version}} -m "Release v{{target_version}}"`
- [ ] 推送代码: `git push origin main`
- [ ] 推送标签: `git push origin v{{target_version}}`

### 4. CI/CD 触发
- [ ] 等待 GitHub Actions 构建完成
- [ ] 验证二进制发布到 GitHub Releases
- [ ] 验证 Docker 镜像推送到 ghcr.io

### 5. 发布后验证
- [ ] 下载并测试新版本二进制
- [ ] 验证安装脚本可用
- [ ] 更新文档站点 (如有)
```

**自动化发布流程：**
```
ltmatrix release patch
        │
        ▼
┌───────────────────────────────────────────────────────┐
│  1. 预检查                                             │
│     ├─ cargo test --all                               │
│     ├─ cargo fmt --check                              │
│     └─ cargo clippy                                   │
├───────────────────────────────────────────────────────┤
│  2. 版本计算                                           │
│     ├─ 读取 Cargo.toml 当前版本                        │
│     ├─ 根据 release type 计算新版本                    │
│     └─ 更新 Cargo.toml & Cargo.lock                   │
├───────────────────────────────────────────────────────┤
│  3. CHANGELOG 生成                                     │
│     ├─ 收集 git commits (since last tag)              │
│     ├─ 分类: feat/fix/docs/refactor                   │
│     └─ 更新 CHANGELOG.md                              │
├───────────────────────────────────────────────────────┤
│  4. Git 提交 & 标签                                    │
│     ├─ git add -A && git commit                       │
│     ├─ git tag -a v{x.y.z}                            │
│     └─ git push origin main --tags                    │
├───────────────────────────────────────────────────────┤
│  5. 监控 CI/CD                                         │
│     ├─ 轮询 GitHub Actions 状态                        │
│     ├─ 等待构建完成 (超时: 30min)                      │
│     └─ 报告发布结果                                    │
└───────────────────────────────────────────────────────┘
```

**GitHub Actions 发布配置 (.github/workflows/release.yml)：**
```yaml
name: Release

on:
  push:
    tags:
      - 'v*'

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
          - target: x86_64-apple-darwin
            os: macos-latest
          - target: aarch64-apple-darwin
            os: macos-latest
          - target: x86_64-pc-windows-msvc
            os: windows-latest

    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: softprops/action-gh-release@v1
        with:
          files: target/${{ matrix.target }}/release/ltmatrix*

  docker:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: docker/login-action@v3
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}
      - uses: docker/build-push-action@v5
        with:
          push: true
          tags: ghcr.io/${{ github.repository }}:latest
```

**配置文件发布设置：**
```toml
# ~/.ltmatrix/config.toml

[release]
# 默认分支
branch = "main"

# 是否自动推送
auto_push = true

# 是否自动创建标签
auto_tag = true

# CI/CD 超时 (秒)
ci_timeout = 1800

# 发布前检查命令
pre_check_commands = [
  "cargo test --all",
  "cargo fmt --check",
  "cargo clippy -- -D warnings"
]

# CHANGELOG 模板
changelog_template = "default"  # default, conventional, custom
```

---

## 技术选型

| 领域 | 库 | 说明 |
|------|-----|------|
| CLI | `clap` | 参数解析 |
| 异步运行时 | `tokio` | 异步任务调度 |
| 序列化 | `serde` | JSON/TOML 处理 |
| 日志 | `tracing` | 结构化日志 |
| 进度条 | `indicatif` | 终端进度展示 |
| 终端输出 | `termcolor` / `anstream` | 彩色输出 |
| 配置 | `config` | 多源配置合并 |
| 错误处理 | `anyhow` / `thiserror` | 错误类型 |
| 子进程 | `tokio::process` | 调用 Agent CLI |
| Git | `git2` | Git 操作 |

---

## 里程碑

| 版本 | 目标 | 关键交付 |
|------|------|---------|
| **v0.1.0** | MVP | Claude 后端、标准模式 7 步流水线、依赖调度、Git 集成、`--resume` |
| **v0.2.0** | 扩展性 | 多 Agent 后端、TOML 配置文件、`--dry-run`、`--on-blocked`、JSON 输出 |
| **v0.3.0** | 质量 | 专家模式（Code Review Agent）、实时进度条（indicatif）、结构化日志 |
| **v1.0.0** | 发布 | `release` 子命令、跨平台二进制、CI/CD 流程、完整文档 |
