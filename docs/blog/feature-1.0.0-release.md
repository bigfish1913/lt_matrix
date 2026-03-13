# lt_matrix 1.0.0 Feature Release: Intelligent Agent Orchestration

我们很高兴地宣布 **lt_matrix 1.0.0** 正式发布！这是一个长期运行代理编排器，专为复杂的软件开发任务而设计。

## 🎯 核心特性

### 1. 智能代理类型编排

lt_matrix 支持四种代理类型，每种类型专注于不同的任务阶段：

| 代理类型 | 职责 | 使用场景 |
|---------|------|---------|
| **Plan** | 任务分解和复杂度评估 | 生成阶段自动分配 |
| **Dev** | 代码实现 | 核心开发任务 |
| **Test** | 编写和运行测试 | Standard/Expert 模式 |
| **Review** | 代码审查 | Expert 模式专属 |

```rust
// 自动根据任务内容分配代理类型
let combined = format!("{} {}", task.title, task.description);
task.agent_type = AgentType::from_keywords(&combined);
```

### 2. 三种执行模式

| 模式 | 特点 | 适用场景 |
|------|------|---------|
| **Fast** | 跳过测试，使用快速模型 | 快速原型开发 |
| **Standard** | 完整 6 阶段流水线 | 日常开发任务 |
| **Expert** | 包含代码审查阶段 | 关键功能、生产代码 |

```bash
# Fast 模式 - 快速迭代
lt_matrix --mode fast "实现 hello world API"

# Standard 模式 - 完整测试
lt_matrix --mode standard "添加用户认证"

# Expert 模式 - 最高质量
lt_matrix --mode expert "重构核心模块"
```

### 3. 优先级调度系统

任务调度现在考虑两个关键因素：

**1. 任务优先级**
- 每个任务有 0-9 的优先级
- 高优先级任务优先执行

**2. 阻塞链优先级提升**
- 如果一个任务阻塞了多个下游任务，它的优先级会自动提升
- 阻塞 3 个下游任务 → +3 优先级提升
- 最大提升上限可配置

```rust
// 计算优先级提升
let downstream_count = count_downstream_tasks(task_id, graph);
let boost = min(downstream_count * config.blocking_boost, config.max_boost);
effective_priority = task.priority + boost;
```

### 4. 关联任务分组

共享上下文的任务会被分组在一起执行，最大化会话复用：

```rust
// 如果 task-1 和 task-2 共享相同的 related_tasks
// 它们会被安排在相邻位置执行
task1.related_tasks = vec!["auth-module"];
task2.related_tasks = vec!["auth-module"]; // 相同上下文
```

### 5. 智能错误处理与重试

**错误分类**
- **可重试错误**: 超时、速率限制、网络错误
- **不可重试错误**: 语法错误、权限拒绝、文件未找到

**指数退避重试**
```
第1次重试: 1秒延迟
第2次重试: 2秒延迟
第3次重试: 4秒延迟 (最大 60秒)
```

```rust
// 自动识别错误类型
let error_class = classify_error(&error_message);
match error_class {
    ErrorClass::Retryable => retry_with_backoff(),
    ErrorClass::NonRetryable => skip_task(),
    ErrorClass::Unknown => retry_with_caution(),
}
```

### 6. 多层内存系统

**项目级内存** (`.ltmatrix/memory/project.json`)
- 项目结构和技术栈
- 编码规范和模式
- 已完成任务历史
- 架构决策记录

**运行级内存** (`.ltmatrix/memory/run-{id}.json`)
- 代理会话状态
- 上下文决策记录
- 任务执行历史
- 会话复用统计

```rust
// 项目内存
let mut project = ProjectMemory::new("my-project");
project.record_completed_task(task);
project.add_decision(decision);

// 运行内存
let mut run = RunMemory::with_mode("standard");
run.record_session(session_id, "dev");
run.record_decision(context_decision);
```

## 🚀 快速开始

### 安装

```bash
cargo install lt_matrix
```

### 基本使用

```bash
# 初始化项目
lt_matrix init

# 运行任务
lt_matrix run "实现用户登录功能"

# 指定模式
lt_matrix run --mode expert "重构数据库层"

# 查看任务状态
lt_matrix status
```

### 配置文件

创建 `.ltmatrix/config.toml`:

```toml
[agents]
max_plan_agents = 1
max_dev_agents = 2
max_test_agents = 1
max_review_agents = 1

[mode.fast]
plan_model = "claude-opus-4-6"
exec_model = "claude-sonnet-4-6"
max_retries = 2

[mode.standard]
max_retries = 3

[mode.expert]
review_model = "claude-opus-4-6"
max_retries = 5
```

## 📊 性能对比

| 指标 | Fast 模式 | Standard 模式 | Expert 模式 |
|-----|----------|---------------|-------------|
| 执行时间 | ~30% | 100% | ~150% |
| 代码质量 | 标准 | 高 | 最高 |
| 测试覆盖 | 无 | 完整 | 完整 + 审查 |
| 推荐场景 | 原型 | 日常开发 | 关键功能 |

## 🔧 技术架构

```
┌─────────────────────────────────────────────────────────────┐
│                        CLI Layer                             │
│                    (main.rs, src/cli/)                       │
├─────────────────────────────────────────────────────────────┤
│                    Configuration Layer                       │
│                    (crates/ltmatrix-config/)                 │
├─────────────────────────────────────────────────────────────┤
│                   Orchestration Layer                        │
│                 (src/pipeline/orchestrator.rs)               │
├─────────────────────────────────────────────────────────────┤
│                       Stage Layer                            │
│        (Generate → Assess → Execute → Test → Verify → Commit)│
├─────────────────────────────────────────────────────────────┤
│                      Agent Layer                             │
│          (Claude, OpenCode, KimiCode, Codex backends)        │
├─────────────────────────────────────────────────────────────┤
│                   Infrastructure Layer                       │
│            (git/, mcp/, workspace/, memory/)                  │
└─────────────────────────────────────────────────────────────┘
```

## 📝 开发路线图

### 已完成 ✅
- [x] 代理类型编排 (Plan/Dev/Test/Review)
- [x] 执行模式 (Fast/Standard/Expert)
- [x] 优先级调度
- [x] 关联任务分组
- [x] 错误处理与重试
- [x] 多层内存系统

### 计划中 🚧
- [ ] Web UI 界面
- [ ] 分布式执行支持
- [ ] 更多代理后端集成

## 🔧 Post-Release Updates (2026-03-14)

After the 1.0.0 release, several integration gaps were identified and fixed. The following features are now fully connected to the pipeline:

### Fixed Issues

1. **Scheduler Integration** - The priority scheduler with blocking boosts and related task grouping is now fully integrated into the execution pipeline. Tasks that block multiple downstream tasks now get priority boosts automatically.

2. **Agent Type Auto-Assignment** - Keyword-based fallback now ensures tasks get the correct agent type even if the LLM omits it. The `from_keywords()` function is now called after task generation to correct any missing assignments.

3. **Unified Memory System** - The new `ProjectMemory` and `RunMemory` systems are now integrated for context injection. Agent prompts include memory from `.ltmatrix/memory/` with fallback to legacy `.claude/memory.md`.

4. **Failure Report Surfacing** - Failed tasks now generate detailed failure reports that are logged prominently and written to `.ltmatrix/failures.log` for post-mortem analysis.

### Technical Details

```rust
// Scheduler integration with priority boosts
let priority_config = PriorityConfig {
    blocking_boost: 1,
    max_boost: 3,
    enable_priority_sorting: true,
    enable_related_grouping: true,
};
let execution_plan = schedule_tasks_with_priority(tasks, &priority_config)?;

// Agent type fallback after LLM generation
for task in &mut tasks {
    if task.agent_type == AgentType::Dev {
        let detected = AgentType::from_keywords(&combined);
        if detected != AgentType::Dev {
            task.agent_type = detected;
        }
    }
}

// Unified memory loading with fallback
let project_memory = load_unified_memory(&config.work_dir).await?;

// Failure report surfacing
if task.error.is_some() {
    let report = generate_failure_report(&task, &task_map);
    warn!("=== FAILURE REPORT ===");
    // ... logs and writes to failures.log
}
```

---

## 🤝 贡献

欢迎贡献！请查看 [CONTRIBUTING.md](CONTRIBUTING.md) 了解详情。

## 📄 许可证

MIT License - 详见 [LICENSE](../LICENSE)

---

**链接**: [GitHub](https://github.com/bigfish1913/lt_matrix) | [文档](https://docs.lt_matrix.dev) | [问题反馈](https://github.com/bigfish1913/lt_matrix/issues)
