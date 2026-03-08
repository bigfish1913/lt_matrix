# lt_matrix 多模型配置指南

lt_matrix 支持多种 AI 模型，包括 Claude、OpenAI、智谱、阿里等。

## 配置文件位置

项目配置：`.ltmatrix/config.toml`
全局配置：`~/.config/ltmatrix/config.toml`

## 配置格式

### Claude 配置

```toml
[agents.claude]
model = "claude-sonnet-4-6"  # 或 claude-opus-4-6, claude-haiku-4-5
timeout = 3600
```

**API 密钥设置：**
- 方式1：环境变量 `ANTHROPIC_API_KEY`
- 方式2：在 `env` 字段中设置
```toml
[agents.claude.env]
ANTHROPIC_API_KEY = "sk-ant-..."
```

### OpenAI 模型配置

```toml
[agents.opencode]
model = "gpt-4"  # 或 gpt-3.5-turbo
command = "opencode"
timeout = 3600

[agents.opencode.env]
OPENAI_API_KEY = "sk-..."
```

### 智谱 GLM 配置

```toml
[agents.kimicode]
model = "glm-4"  # 或 glm-3-turbo
command = "kimi-code"
timeout = 3600

[agents.kimicode.env]
ZHIPU_API_KEY = "your-zhipu-api-key"
```

### 阿里通义千问配置

```toml
[agents.codex]
model = "qwen-max"  # 或 qwen-plus, qwen-turbo
command = "codex"
timeout = 3600

[agents.codex.env]
DASHSCOPE_API_KEY = "your-ali-api-key"
```

## 环境变量优先级

1. 配置文件中的 `env` 字段（最高优先级）
2. 系统环境变量
3. 全局配置文件

## 使用指定模型

```bash
# 使用 Claude（默认）
lt_matrix --agent claude "实现用户登录功能"

# 使用 OpenAI
lt_matrix --agent opencode "实现用户登录功能"

# 使用智谱
lt_matrix --agent kimicode "实现用户登录功能"

# 使用阿里
lt_matrix --agent codex "实现用户登录功能"
```

## 完整配置示例

```toml
# 默认 agent
default = "claude"

# Agent 配置
[agents.claude]
model = "claude-sonnet-4-6"
timeout = 3600

[agents.opencode]
model = "gpt-4"
timeout = 3600

[agents.kimicode]
model = "glm-4"
timeout = 3600

[agents.codex]
model = "qwen-max"
timeout = 3600

# 模式配置
[modes.fast]
max_retries = 2

[modes.standard]
max_retries = 3

[modes.expert]
max_retries = 5
review_model = "claude-opus-4-6"
```
