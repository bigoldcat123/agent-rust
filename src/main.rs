use agent::{
    AgentOutputPart, Message, OpenAI, Provider, RequestBuilder, Tool, ToolCall, ToolExecutor,
    ToolFn, ToolOutput, ToolRegistry, error::Error,
};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, stdin, stdout};
use tokio::process::Command;

fn weather_tool() -> (Tool, impl ToolExecutor) {
    (
        Tool::new(
            "get_weather",
            Some("get the weather info ".to_string()),
            Some(json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA",
                    }
                },
                "required": ["location"]
            })),
            None,
        ),
        ToolFn::new(|toolcall: ToolCall| async {
            Ok(ToolOutput::new(
                toolcall.id,
                "天气超级棒！ 温度12°C 湿度30%",
            ))
        }),
    )
}

fn shell_tool() -> (Tool, impl ToolExecutor) {
    (
        Tool::new(
            "shell",
            Some("Execute a shell command and return stdout, stderr, and exit status.".to_string()),
            Some(json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    }
                },
                "required": ["command"]
            })),
            None,
        ),
        ToolFn::new(|toolcall: ToolCall| async move {
            let args: Value = serde_json::from_str(&toolcall.arguments).map_err(|source| {
                Error::InvalidToolArguments {
                    name: toolcall.name.clone(),
                    source,
                }
            })?;
            let command =
                args.get("command")
                    .and_then(Value::as_str)
                    .ok_or_else(|| Error::ToolFailed {
                        name: toolcall.name.clone(),
                        message: "missing string argument `command`".to_string(),
                    })?;
            let output = Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .await
                .map_err(|e| Error::ToolFailed {
                    name: toolcall.name.clone(),
                    message: e.to_string(),
                })?;
            Ok(ToolOutput::new(
                toolcall.id,
                json!({
                    "success": output.status.success(),
                    "status": output.status.code(),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                })
                .to_string(),
            ))
        }),
    )
}

fn ask_user_tool() -> (Tool, impl ToolExecutor) {
    (
        Tool::new(
            "ask_user",
            Some(
                "Ask the user a question in the terminal and wait for their answer. Optional preset answers can be provided."
                    .to_string(),
            ),
            Some(json!({
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "The question to ask the user."
                    },
                    "options": {
                        "type": "array",
                        "description": "Optional preset answers. The user can type a number or enter a custom answer.",
                        "items": {
                            "type": "string"
                        }
                    }
                },
                "required": ["question"]
            })),
            None,
        ),
        ToolFn::new(|toolcall: ToolCall| async move {
            let args: Value = serde_json::from_str(&toolcall.arguments).map_err(|source| {
                Error::InvalidToolArguments {
                    name: toolcall.name.clone(),
                    source,
                }
            })?;
            let question = args
                .get("question")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::ToolFailed {
                    name: toolcall.name.clone(),
                    message: "missing string argument `question`".to_string(),
                })?
                .to_string();
            let options = args
                .get("options")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            let mut out = stdout();
            out.write_all(format!("\n{}\n", question).as_bytes())
                .await
                .map_err(|e| Error::ToolFailed {
                    name: toolcall.name.clone(),
                    message: e.to_string(),
                })?;
            for (index, option) in options.iter().enumerate() {
                out.write_all(format!("  {}. {}\n", index + 1, option).as_bytes())
                    .await
                    .map_err(|e| Error::ToolFailed {
                        name: toolcall.name.clone(),
                        message: e.to_string(),
                    })?;
            }
            out.write_all(b"> ").await.map_err(|e| Error::ToolFailed {
                name: toolcall.name.clone(),
                message: e.to_string(),
            })?;
            out.flush().await.map_err(|e| Error::ToolFailed {
                name: toolcall.name.clone(),
                message: e.to_string(),
            })?;

            let mut input = String::new();
            BufReader::new(stdin())
                .read_line(&mut input)
                .await
                .map_err(|e| Error::ToolFailed {
                    name: toolcall.name.clone(),
                    message: e.to_string(),
                })?;
            let raw_answer = input.trim().to_string();
            let selected_index = raw_answer
                .parse::<usize>()
                .ok()
                .and_then(|index| index.checked_sub(1))
                .filter(|index| *index < options.len());
            let answer = selected_index
                .and_then(|index| options.get(index).cloned())
                .unwrap_or_else(|| raw_answer.clone());

            Ok(ToolOutput::new(
                toolcall.id,
                json!({
                    "question": question,
                    "answer": answer,
                    "raw_answer": raw_answer,
                    "selected_index": selected_index,
                })
                .to_string(),
            ))
        }),
    )
}

#[tokio::main]
async fn main() {
    let mut tool_registry = ToolRegistry::new();
    let (tool, tool_executor) = weather_tool();
    tool_registry.insert_executor(tool, tool_executor);
    let (tool, tool_executor) = shell_tool();
    tool_registry.insert_executor(tool, tool_executor);
    let (tool, tool_executor) = ask_user_tool();
    tool_registry.insert_executor(tool, tool_executor);
    let messages = vec![
        Message::system(r#"you are a helpful agent，每当你有不明白的地方，你总是会调用ask_user 去询问。"#),
        Message::user_text("请你问我几个问题，然后总结出我是一个什么人。"),
    ];
    let req = RequestBuilder::default()
        .modle("deepseek-v4-flash")
        .messages(messages)
        .tools(tool_registry.tools())
        .build()
        .unwrap();
    let (mut client, mut rx) = OpenAI::new().with_tool_executor(tool_registry).with_tx();
    tokio::spawn(async move {
        let _res = client.run_for_result(req).await;
        println!("\n{:?}", _res);
    });
    let mut is_reasoning = false;
    let mut stdout = stdout();
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentOutputPart::Content(text) => {
                if is_reasoning {
                    is_reasoning = false;
                    println!("\n -> answer 🤓")
                }
                stdout.write_all(text.as_bytes()).await.unwrap();
                stdout.flush().await.unwrap();
                // print!("{}", text)
            }
            AgentOutputPart::Reasoning(r) => {
                if !is_reasoning {
                    println!("\n -> reasoning 🤔");
                    is_reasoning = true
                }
                stdout.write_all(r.as_bytes()).await.unwrap();
                stdout.flush().await.unwrap();
            }
            AgentOutputPart::Tool(tools) => {
                for t in tools {
                    println!("\ntool call -> {}", t.name);
                }
            }
        }
    }
}
