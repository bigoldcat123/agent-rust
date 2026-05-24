use agent::{
    AgentOutputPart, Message, OpenAI, Provider, RequestBuilder, Tool, ToolCall, ToolExecutor,
    ToolFn, ToolOutput, ToolRegistry,
};
use serde_json::json;
use tokio::io::{AsyncWriteExt, stdout};

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
        ToolFn::new(|toolcall: ToolCall| async { Ok(ToolOutput::new(toolcall.id, "天气超级棒！ 温度12°C 湿度30%")) }),
    )
}

#[tokio::main]
async fn main() {
    let mut tool_registry = ToolRegistry::new();
    let (tool, tool_executor) = weather_tool();
    tool_registry.insert_executor(tool, tool_executor);
    let messages = vec![
        Message::system(r#"任何消息都以json的形式返回给我！例如{"answer":"your answer"}"#),
        Message::user_text("北京的天气怎么样？"),
    ];
    let req = RequestBuilder::default()
        .modle("deepseek-v4-flash")
        .messages(messages)
        .tools(tool_registry.tools())
        .extra(json!({
            "response_format":{
                "type":"json_object"
            }
        }))
        .build()
        .unwrap();
    let (mut client, mut rx) = OpenAI::with_tool_registory(req, tool_registry);

    tokio::spawn(async move {
        let _res = client.run_for_result().await;
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
                println!("\n{tools:?}");
            }
        }
    }
}
