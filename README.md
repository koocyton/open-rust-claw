# rust-bot

一个 Rust 编写的常驻进程，监听 Telegram 频道消息，通过 MCP（Model Context Protocol）协议与 AI 交互，自动将自然语言指令转化为可执行的 shell 命令并执行。

## 架构

```
Telegram 频道消息
       │
       ▼
  ┌──────────┐
  │ rust-bot │  (常驻进程)
  └──────────┘
       │
       ▼
  ┌──────────┐
  │ MCP 服务 │  (通过 stdio JSON-RPC 通信)
  └──────────┘
       │
       ▼
  返回命令列表
       │
       ▼
  ┌──────────┐
  │ 命令执行  │  (shell 执行)
  └──────────┘
       │
       ▼
  结果回传 Telegram
```

## 工作流程

1. **启动** — 读取 `config.toml`，启动 MCP 服务端子进程，完成 MCP 握手
2. **监听** — 通过 Telegram Bot API 长轮询监听频道/群组消息
3. **分析** — 将消息内容发送给 MCP 服务端，请求生成命令执行计划
4. **执行** — 按顺序执行 MCP 返回的命令列表（任一失败则停止）
5. **反馈** — 将执行结果回传到 Telegram 频道

## 快速开始

### 前置条件

- Rust 1.70+
- 一个 Telegram Bot Token（从 [@BotFather](https://t.me/BotFather) 获取）
- 一个 MCP 服务端（如 `@modelcontextprotocol/server-everything`）

### 构建

```bash
cargo build --release
```

### 配置

```bash
cp config.example.toml config.toml
# 编辑 config.toml，填入你的 Bot Token 和 MCP 配置
```

### 运行

```bash
# 使用默认配置文件 config.toml
./target/release/rust-bot

# 指定配置文件路径
./target/release/rust-bot /path/to/config.toml

# 开启 debug 日志
RUST_LOG=debug ./target/release/rust-bot
```

## 配置说明

| 配置项 | 说明 |
|--------|------|
| `telegram.bot_token` | Telegram Bot API Token |
| `telegram.allowed_chat_ids` | 允许的聊天 ID 白名单，空数组表示不限制 |
| `mcp.command` | MCP 服务端启动命令 |
| `mcp.args` | MCP 服务端启动参数 |
| `mcp.env` | 传递给 MCP 服务端的环境变量 |
| `executor.working_dir` | 命令执行的工作目录 |
| `executor.timeout_secs` | 单条命令超时时间（秒） |
| `executor.echo_result` | 是否回传执行结果到 Telegram |

## MCP 协议

本程序作为 MCP 客户端，通过 stdio（标准输入/输出）与 MCP 服务端通信，使用 JSON-RPC 2.0 协议：

- `initialize` — 握手，交换能力信息
- `tools/list` — 获取服务端可用工具列表
- `tools/call` — 调用工具，传入 Telegram 消息内容，获取命令列表

## 安全注意事项

- **务必配置 `allowed_chat_ids`**，限制只有授权的频道/用户才能触发命令执行
- 该程序会在服务器上执行任意 shell 命令，请确保运行环境安全
- 建议使用受限用户运行，避免使用 root
