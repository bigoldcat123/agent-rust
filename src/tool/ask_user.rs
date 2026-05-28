use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, stdin, stdout};

use crate::{
    Tool, ToolCall,
    error::Error,
    tool::{RegisteredTool, SharedRegisteredTool, ToolFn, ToolOutput},
};

pub fn ask_user_tool() -> SharedRegisteredTool {
    RegisteredTool::new(
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
