# Feature 1.0.0 规划

> Long-Running Agent Orchestrator 1.0.0 版本功能规划文档

---

## 核心概念

### Agent 类型定义

系统在任务拆分阶段就会标记任务类型，调度器根据类型分配对应 Agent。

| 类型 | 标识 | 职责 | 适用任务示例 |
|------|------|------|--------------|
| **规划 Agent** | `plan` | 任务拆分与规划 | 分析目标、生成任务列表、复杂度评估 |
| **开发 Agent** | `dev` | 代码编写 | 实现功能、重构、修复 bug、编写文档 |
| **测试 Agent** | `test` | 测试相关 | 编写测试、运行测试、生成覆盖率报告 |
| **审查 Agent** | `review` | 代码审查 | Expert 模式下的代码质量检查、安全审计 |

### 任务结构扩展

```json
{
  "id": "task-001",
  "title": "实现用户登录功能",
  "description": "创建登录 API 端点和表单验证",
  "agent_type": "dev",           // 执行该任务的 Agent 类型
  "priority": 5,                 // 优先级 0-9
  "related_tasks": ["task-002"], // 关联任务
  "depends_on": [],              // 依赖任务
  "status": "Pending",

  // 新增：任务资源指定
  "resources": {
    "docs": [                    // 参考文档（详细业务逻辑）
      "docs/login-spec.md",
      "docs/api-contract.md"
    ],
    "skills": [                  // 使用的技能
      "frontend-design",
      "autotest"
    ],
    "mcp_tools": [               // 使用的 MCP 工具
      "playwright",
      "web-reader"
    ]
  }
}
```

### 任务资源说明

| 字段 | 类型 | 说明 | 示例 |
|------|------|------|------|
| `docs` | `string[]` | 参考文档路径，包含详细业务逻辑 | `["docs/api-spec.md"]` |
| `skills` | `string[]` | 需要使用的技能名称 | `["frontend-design", "autotest"]` |
| `mcp_tools` | `string[]` | 需要使用的 MCP 工具 | `["playwright", "web-reader"]` |

> **使用场景**：
> - 复杂业务逻辑：指定详细的设计文档，Agent 会先阅读文档再执行
> - 需要 UI 生成：指定 `frontend-design` skill
> - 需要 E2E 测试：指定 `autotest` skill + `playwright` MCP
> - 需要读取网页：指定 `web-reader` MCP

### 项目开发规范

系统支持项目级开发规范，在**每个任务执行时自动注入**到对应类型 Agent 的上下文中。

**规范文件位置**：
```
.ltagents/guidelines/            # 按 Agent 类型分文件（推荐）
├── _common.md                   # 通用规范（所有 Agent 共享）
├── plan.md                      # Plan Agent 规范
├── dev.md                       # Dev Agent 规范
├── test.md                      # Test Agent 规范
└── review.md                    # Review Agent 规范

# 或单文件方式（按章节区分）
.ltagents/guidelines.md          # 包含所有规范的单一文件
```

**规范文件示例**：

**`.ltagents/guidelines/_common.md`**（通用规范）：
```markdown
# 通用规范

## 代码风格
- 使用 UTF-8 编码
- 文件末尾保留一个空行

## 安全规范
- 密码必须加密存储
- 敏感信息不能出现在日志中
```

**`.ltagents/guidelines/dev.md`**（开发规范）：
```markdown
# 开发规范

## 数值处理
- 碰到小数，保留两位有效数字
- 金额使用"分"作为单位，避免浮点精度问题

## 接口调用
- 调用前打印日志：请求参数、时间戳
- 调用后打印日志：响应结果、耗时
- 异常时打印：错误信息、堆栈

## 命名规范
- 变量：snake_case
- 常量：SCREAMING_SNAKE_CASE
- 函数：动词开头，如 get_user_by_id
```

**`.ltagents/guidelines/test.md`**（测试规范）：
```markdown
# 测试规范

## 覆盖率要求
- 核心模块：≥ 80%
- 工具函数：≥ 90%
- 整体项目：≥ 70%

## 测试命名
- 单元测试：test_<function>_<scenario>
- 集成测试：test_<feature>_<scenario>

## 测试数据
- 使用 fixture 文件，不要硬编码
- 敏感数据使用 mock
```

**`.ltagents/guidelines/review.md`**（审查规范）：
```markdown
# 审查规范

## 检查清单
- [ ] 代码是否符合命名规范
- [ ] 是否有安全漏洞（SQL注入、XSS等）
- [ ] 是否有性能问题
- [ ] 错误处理是否完善

## 关注点
- 复杂度：单个函数不超过 50 行
- 重复代码：DRY 原则
- 注释：关键逻辑必须有注释
```

**`.ltagents/guidelines/plan.md`**（规划规范）：
```markdown
# 规划规范

## 任务拆分原则
- 单个任务粒度：1-4 小时
- 最大深度：3 层
- 避免循环依赖

## 复杂度评估
- 简单：仅修改配置或文档
- 中等：新增功能或重构
- 复杂：涉及架构变更
```

**配置方式**：
```toml
[project]
# 规范目录（可选，默认 .ltagents/guidelines/）
guidelines_dir = ".ltagents/guidelines"

# 是否启用全局规范合并
merge_global_guidelines = true
```

**注入机制**：
1. 任务执行前，根据 `agent_type` 读取对应规范文件
2. 先加载 `_common.md`（通用规范）
3. 再加载 `<agent_type>.md`（特定规范）
4. 合并后作为 system prompt 的一部分注入 Agent

### 类型匹配规则

| 规则 | 说明 |
|------|------|
| 精确匹配 | 任务 `agent_type` 必须与 Agent 类型一致 |
| 类型继承 | 关联任务默认继承父任务的 `agent_type`，可覆盖 |
| 跨类型依赖 | `dev` 任务可依赖 `test` 任务，但需等待其完成 |
| 角色隔离 | 不同类型 Agent 不能互用，保证专业性 |

### 执行模式与 Agent 配置

不同执行模式下，Agent 的使用策略和任务流程有所不同：

| 模式 | Plan 模型 | 执行模型 | Review 模型 | 启用的 Agent | 任务流程 | 适用场景 |
|------|-----------|----------|-------------|--------------|----------|----------|
| **Fast** | Opus | Sonnet | - | `plan` + `dev` | Generate → Assess → Execute → Verify → Commit | 快速原型、简单任务 |
| **Standard** | Opus | Sonnet | - | `plan` + `dev` + `test` | Generate → Assess → Execute → Test → Fix → Verify → Commit | 日常开发、标准流程 |
| **Expert** | Opus | Sonnet | Opus | `plan` + `dev` + `test` + `review` | Generate → Assess → Execute → Test → Fix → Review → Fix → Verify → Commit | 关键功能、生产代码 |

> **默认模型策略**（可用户自定义）：
> - Plan Agent：Opus（规划需要更强智能）
> - 执行 Agent（dev/test）：Sonnet（平衡速度和质量）
> - Review Agent：Opus（审查需要更深入的代码理解）

#### 模式差异详解

**Fast 模式**
- 跳过测试和修复阶段，`test` 类型任务被标记为 Skipped
- 不创建测试 Agent，节省资源
- Plan 用 Opus，执行用 Sonnet
- 最大深度限制为 2
- 适合：快速验证想法、非关键代码

**Standard 模式**
- 完整流程，包含测试和修复
- `test` 任务正常执行，失败后生成 `fix` 任务
- Plan 用 Opus，执行用 Sonnet
- 最大深度限制为 3
- 适合：日常开发、一般功能实现

**Expert 模式**
- 增加 Review 阶段（区别于 Standard）
- 测试失败 → Fix，Review 发现问题 → 再 Fix
- Plan 用 Opus，执行用 Sonnet，Review 用 Opus（默认，可配置）
- 更高的重试次数（默认 5 次）
- 适合：核心模块、安全相关代码、生产发布前

#### 模式配置示例

```toml
# 模式特定配置（以下为默认值，用户可自定义）
[mode.fast]
plan_model = "claude-opus-4-6"       # 规划模型
exec_model = "claude-sonnet-4-6"     # 执行模型（dev/test）
enabled_agents = ["plan", "dev"]
max_depth = 2
skip_test = true
max_retries = 2

[mode.standard]
plan_model = "claude-opus-4-6"
exec_model = "claude-sonnet-4-6"
enabled_agents = ["plan", "dev", "test"]
max_depth = 3
max_retries = 3

[mode.expert]
plan_model = "claude-opus-4-6"
exec_model = "claude-sonnet-4-6"
review_model = "claude-opus-4-6"     # 审查模型（默认 Opus，可配置为 Sonnet）
enabled_agents = ["plan", "dev", "test", "review"]
max_depth = 3
max_retries = 5
auto_fix_after_review = true
```

---

## 新增功能

> 按执行流程排序：启动 → 生成 → 校验 → 调度 → 执行 → 异常处理 → 持久化

### 1. 启动恢复检测
**阶段：启动**

- 启动时检测是否存在未完成任务
- 若有：询问用户是否进入 resume 模式
- 若无或用户选择不恢复：清理旧任务，重新生成任务计划

---

### 2. 任务拆分日志
**阶段：生成**

- 任务拆分时输出详细日志
- 记录拆分理由：
  - 复杂度评估结果
  - 依赖分析过程
  - 拆分策略选择
- **任务类型标记**：
  - 根据 task title 和 description 自动判断 `agent_type`
  - 关键词匹配规则：
    - `plan`: "分析"、"规划"、"拆分"、"评估"
    - `dev`: "实现"、"编写"、"修复"、"重构"、"创建"
    - `test`: "测试"、"验证"、"覆盖率"、"断言"
    - `review`: "审查"、"检查"、"审计"、"评审"
  - 允许手动指定类型（通过配置或注释）

---

### 3. 依赖关系校验
**阶段：校验**

- 生成任务后校验依赖图完整性
- 检测条件：
  - **循环依赖**: A→B→C→A 形成闭环
  - **不可达任务**: 依赖了不存在的任务
  - **孤立节点**: 非入口/出口的中间孤立点（无依赖且无后续任务）
- 校验失败时：报告错误详情，拒绝执行

---

### 4. 调试模式流程图
**阶段：校验（辅助）**

- debug 模式下输出任务依赖图
- 输出格式：
  - mermaid 源码（默认，直接输出到控制台）
  - mermaid png 图片（可选，需安装 mermaid-cli 或使用在线渲染）
- 内容包含：
  - 节点 ID 和标题
  - 简要描述
  - 当前状态（Pending/InProgress/Completed/Failed）

---

### 5. Agent 池化管理
**阶段：调度**

- 预创建 Agent 实例，避免重复创建开销
- 角色分类（与任务类型对应）：
  - **规划 Agent (`plan`)**: 任务拆分与复杂度评估
  - **开发 Agent (`dev`)**: 执行代码编写
  - **测试 Agent (`test`)**: 编写和运行测试（Standard/Expert 模式）
  - **审查 Agent (`review`)**: 代码审查（仅 Expert 模式）
- **模式感知初始化**：
  - 根据当前模式只初始化需要的 Agent 类型
  - Fast 模式：仅 `plan` + `dev`
  - Standard 模式：`plan` + `dev` + `test`
  - Expert 模式：`plan` + `dev` + `test` + `review`
- **类型匹配调度**：
  - 从任务队列取任务时，读取 `agent_type` 字段
  - 若任务类型不在当前模式启用列表中，标记为 Skipped-ModeDisabled
  - 从对应类型的 Agent 池中获取空闲 Agent
  - 若该类型 Agent 全部忙碌，等待或创建新实例（不超过限制）
- 并行度控制：
  - 配置项：`max_parallel_agents`（默认：CPU 核心数）
  - 每种角色独立限制：`max_plan_agents`, `max_dev_agents`, `max_test_agents`, `max_review_agents`
- 实现：
  - Agent 空闲队列（按 `agent_type` 分组）
  - Agent 忙队列
  - 获取/释放机制

---

### 6. 上下文复用与任务调度
**阶段：调度**

- 同一 Agent 尽量执行相关联任务
- **任务关联字段**：任务拆分后增加 `related_tasks` 字段
  - 执行完当前任务后，检查关联任务是否已执行
  - 若上下文空间足够，当前 Agent 优先执行关联任务
- **任务优先级**：
  - 优先级字段：`priority`（0-9，默认 5）
  - 高优先级任务优先调度
  - 阻塞链上的任务自动提升优先级
- 复用条件：
  - 相同依赖链
  - 涉及相关文件
- 不可复用时：清理上下文，释放资源
- 清理策略：LRU 淘汰 或 手动触发

---

### 7. 失败任务处理
**阶段：异常处理**

- 上级依赖失败时的处理策略：
  1. **尝试跳过**: 标记为 Skipped，继续执行其他任务
  2. **尝试替代方案**: 寻找备选执行路径
  3. **记录详情**: 失败原因、影响范围、受影响任务列表
- 生成失败报告，供用户决策

---

### 8. 任务重试与超时
**阶段：异常处理**

- **重试机制**：
  - 可重试错误：网络超时、API 限流、临时资源不可用
  - 不可重试错误：语法错误、配置错误、权限不足
  - 重试策略：指数退避（1s, 2s, 4s, 8s），最大重试次数可配置
- **超时控制**：
  - 单任务超时：`task_timeout`（默认 3600s）
  - 规划阶段超时：`plan_timeout`（默认 120s）
  - 超时后标记为 Failed，记录超时原因
  - 可配置超时后的重试行为

---

### 9. 任务执行留痕
**阶段：持久化**

- 记录任务执行摘要（输入/输出/耗时）
- 检测重复任务：
  - 相同 goal
  - 相同 context（文件状态、依赖）
- 重复任务处理：
  - 直接跳过（标记为 Skipped-Duplicate）
  - 或复用已有结果

---

## 优化项

### 1. 记忆体系
**整合上下文记忆与项目记忆**

- **项目级记忆**: `.ltagents/memory/project.json`
  - 项目结构
  - 技术栈
  - 编码规范
  - 已完成任务摘要
  - 失败教训
- **CLI 记忆集成**:
  - 读取 `.claude/memory.md`（Claude Code）
  - 读取其他 CLI 工具的记忆文件
- **全局记忆**: `~/.ltmatrix/memory/global.json`
  - 通用最佳实践
  - 常见问题解决方案
- **运行记忆**: `.ltagents/memory/run-{session-id}.json`
  - 本次运行的上下文
  - Agent 会话记录
- **目的**：避免重复生成相同任务，保持上下文连贯

---

### 2. 执行隔离
- 所有产物放入 `.ltagents/` 目录
- 不污染用户工作区
- 提供清理命令：`ltmatrix clean`

---

### 3. 目录结构规范
```
.ltagents/
├── tasks/              # 任务 JSON 文件
│   ├── manifest.json   # 运行元数据
│   └── task-*.json     # 单个任务文件
├── logs/               # 运行日志
│   ├── run.log         # 主日志
│   └── debug.log       # 调试日志
├── memory/             # 记忆文件
│   ├── project.json    # 项目记忆
│   └── run-*.json      # 运行记忆
├── test/               # 测试产物
│   ├── screenshots/    # 测试截图
│   └── coverage/       # 覆盖率报告
├── cache/              # 缓存数据
│   └── agent-responses/# Agent 响应缓存
└── reports/            # 输出报告
    ├── final-report.md # 最终报告
    └── metrics.json    # 指标数据
```

---

### 4. Git 忽略配置
- 自动添加 `.ltagents/` 到 `.gitignore`
- 配置选项：
  ```toml
  [behavior]
  auto_gitignore = true  # 默认 true
  ```
- 若用户选择禁用，则不自动修改 `.gitignore`

---

## 配置示例

```toml
# .ltmatrix/config.toml

[execution]
max_parallel_agents = 4        # 最大并行 Agent 数
task_timeout = 3600            # 单任务超时（秒）
plan_timeout = 120             # 规划超时（秒）
max_retries = 3                # 最大重试次数

[agents]
max_plan_agents = 1            # 规划 Agent 最大数
max_dev_agents = 2             # 开发 Agent 最大数
max_test_agents = 1            # 测试 Agent 最大数
max_review_agents = 1          # 审查 Agent 最大数

[behavior]
auto_gitignore = true          # 自动添加 .gitignore
context_cleanup = "lru"        # 上下文清理策略：lru | manual
```

---

## 待新增任务

基于本功能规划，建议新增以下任务：

### 1. Agent 类型系统
- 定义 `AgentType` 枚举（plan/dev/test/review）
- 扩展 Task 结构，增加 `agent_type` 字段
- 实现任务类型自动识别（关键词匹配）
- 创建 `src/agent/types.rs`

### 2. 执行模式系统
- 定义 `ExecutionMode` 枚举（fast/standard/expert）
- 实现模式配置加载和验证
- 模式与 Agent 类型的映射关系
- 创建 `src/config/mode.rs`

### 3. Agent Pool 实现
- 创建 `src/agent/pool.rs`
- 实现 Agent 池化管理
- 支持角色分类和复用
- 模式感知初始化
- 类型匹配调度逻辑
- 并行度限制

### 4. 任务调度增强
- 任务优先级支持
- 关联任务优先调度
- 阻塞链优先级提升
- 类型匹配调度
- 模式禁用任务跳过

### 5. 重试与超时机制
- 可重试/不可重试错误分类
- 指数退避重试
- 超时检测和处理
- 模式特定重试配置

### 6. 记忆系统增强
- 项目记忆持久化
- CLI 记忆文件读取
- 记忆检索和注入

### 7. 任务去重检测
- 任务指纹计算
- 重复任务识别
- 结果复用机制