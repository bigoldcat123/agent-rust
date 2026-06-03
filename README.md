# Agent

一个用 Rust 编写的轻量级 Agent 框架，用来组合大模型 Provider、消息请求、工具调用、流式输出和多步骤工作流。

当前项目默认使用 `OpenAI` provider 封装 `async-openai` 的 chat completion 流式接口，并内置了几个示例工具：

- `ask_user_tool`：在终端中向用户提问并读取回答
- `weather_tool`：示例天气工具
- `shell_tool`：shell 工具

## Requirements

- Rust 2024 edition
- Tokio runtime
- 可用的大模型 API 环境变量：
  - `OPENAI_BASE_URL`
  - `OPENAI_API_KEY`

## Quick Start

先设置运行所需的环境变量：

```bash
export OPENAI_BASE_URL="https://your-api-base-url"
export OPENAI_API_KEY="your-api-key"
```

运行主程序：

```bash
cargo run
```

运行两个 step 用例：

```bash
cargo run --example step_info_weather
cargo run --example step_switch
```


## Basic Usage

构造请求并调用 provider：

```rust
use agent::{Client, Message, Provider, RequestBuilder};

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let req = RequestBuilder::default()
        .messages(vec![
            Message::system("你是一个简洁的助手。"),
            Message::user_text("介绍一下这个项目。"),
        ])
        .build()
        .map_err(|e| agent::error::Error::RequestBuild {
            message: e.to_string(),
        })?;

    let mut client = Client::new();
    let output = client.run_for_result(req).await?;

    if let Some((content, _reasoning)) = output.get_last_assistant_message() {
        println!("{content}");
    }

    Ok(())
}
```

## Tools

工具通过 `RegisteredTool` 注册，并放进 `RequestBuilder::tools(...)` 中。模型触发 tool call 后，`OpenAI` provider 会执行对应工具，并把工具结果追加回对话继续运行。

示例：

```rust
use agent::{
    Client, Message, Provider, RequestBuilder,
    tool::ask_user_tool,
};

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let req = RequestBuilder::default()
        .messages(vec![Message::user_text("问我一个问题，然后总结我的回答。")])
        .tools(vec![ask_user_tool()])
        .build()
        .map_err(|e| agent::error::Error::RequestBuild {
            message: e.to_string(),
        })?;

    let mut client = Client::new();
    let output = client.run_for_result(req).await?;
    println!("{output:?}");

    Ok(())
}
```

## Step Workflow

`agent::step` 提供一组可组合的工作流构件：

- `Runner`：统一的步骤执行 trait
- `Layer`：把一个步骤包装到另一个步骤外层
- `WorkFlowBuilder`：按顺序组合 layer
- `Switch`：根据输入切换分支
- `JsonOutput`：把最后输入交给模型并要求 JSON 输出
- `source_code!`：快速声明一个模型驱动的 step layer

查看完整用例：

- `examples/step_info_weather.rs`
- `examples/step_switch.rs`

## Project Structure

```text
src/
  agent/       Agent wrapper，例如信息收集、规划、匹配
  provider/    Provider trait 和 OpenAI provider 实现
  tool/        工具定义、注册表和内置工具
  step.rs      多步骤 workflow 构件与 source_code! 宏
  message.rs   Message / ToolCall 类型
  error.rs     项目错误类型
  util/        TUI 流式输出辅助

examples/
  step_info_weather.rs
  step_switch.rs
```

## Notes

这个项目还处在实验阶段，API 命名和模块边界可能继续调整。当前更适合作为 Agent 工作流、tool call 和 provider 组合的原型框架使用。
