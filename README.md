# Agent

English | [中文](README.zh-CN.md)

A lightweight Rust agent framework for composing LLM providers, message requests, tool calls, streaming output, and multi-step workflows.

The project currently uses the `OpenAI` provider to wrap the `async-openai` chat completion streaming API, and includes several built-in example tools:

- `ask_user_tool`: asks the user a question in the terminal and waits for an answer
- `weather_tool`: sample weather tool
- `shell_tool`: shell tool

## Requirements

- Rust 2024 edition
- Tokio runtime
- Required LLM API environment variables:
  - `OPENAI_BASE_URL`
  - `OPENAI_API_KEY`

The project uses a GitHub dependency for `async-openai`:

```toml
async-openai = { git = "https://github.com/bigoldcat123/async-openai", features = ["chat-completion"] }
```

Cargo can fetch this dependency directly after cloning the project.

## Quick Start

Set the required environment variables first:

```bash
export OPENAI_BASE_URL="https://your-api-base-url"
export OPENAI_API_KEY="your-api-key"
```

Run the main program:

```bash
cargo run
```

Run the two step examples:

```bash
cargo run --example step_info_weather
cargo run --example step_switch
```

## Basic Usage

Build a request and call a provider:

```rust
use agent::{Client, Message, Provider, RequestBuilder};

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let req = RequestBuilder::default()
        .messages(vec![
            Message::system("You are a concise assistant."),
            Message::user_text("Introduce this project."),
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

Tools are registered with `RegisteredTool` and passed into `RequestBuilder::tools(...)`. When the model emits a tool call, the `OpenAI` provider runs the matching tool, appends the tool result back into the conversation, and continues the request.

Example:

```rust
use agent::{
    Client, Message, Provider, RequestBuilder,
    tool::ask_user_tool,
};

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let req = RequestBuilder::default()
        .messages(vec![Message::user_text("Ask me a question, then summarize my answer.")])
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

`agent::step` provides composable workflow building blocks:

- `Runner`: the shared execution trait for steps
- `Layer`: wraps one step around another step
- `WorkFlowBuilder`: composes layers in order
- `Switch`: chooses a branch from the input
- `JsonOutput`: sends the final input to the model and asks for JSON output
- `source_code!`: quickly declares an LLM-driven step layer

See the complete examples:

- `examples/step_info_weather.rs`
- `examples/step_switch.rs`

## Project Structure

```text
src/
  agent/       Agent wrappers, such as info collection, planning, and matching
  provider/    Provider trait and OpenAI provider implementation
  tool/        Tool definitions, registry, and built-in tools
  step.rs      Multi-step workflow primitives and source_code! macro
  message.rs   Message / ToolCall types
  error.rs     Project error types
  util/        TUI streaming output helper

examples/
  step_info_weather.rs
  step_switch.rs
```

## Development

```bash
cargo fmt
cargo clippy
cargo clippy --examples
cargo test --no-run
```

## Notes

This project is still experimental. API names and module boundaries may continue to change. It is currently best suited as a prototype framework for agent workflows, tool calls, and provider composition.
