use serde_json::{Value, json};
use tokio::process::Command;

use crate::{
    Tool, ToolCall,
    error::Error,
    tool::{ToolExecutor, ToolFn, ToolOutput},
};

pub fn shell_tool() -> (Tool, impl ToolExecutor) {
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
