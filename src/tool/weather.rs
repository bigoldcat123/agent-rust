use serde_json::json;

use crate::{
    Tool, ToolCall,
    tool::{RegisteredTool, SharedRegisteredTool, ToolFn, ToolOutput},
};

pub fn weather_tool() -> SharedRegisteredTool {
    RegisteredTool::new(
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
