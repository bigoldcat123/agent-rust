use agent::{AgentOutputPart, Message, OpenAI, Provider, RequestBuilder, Tool};
use serde_json::json;
use tokio::io::{AsyncWriteExt, stdout};

#[tokio::main]
async fn main() {
    let tools = vec![Tool::new(
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
    )];
    let messages = vec![
        Message::system(r#"you are a man!"#),
        Message::user_text("北京的天气怎么样？"),
    ];
    let req = RequestBuilder::default()
        .modle("deepseek-v4-flash")
        .messages(messages)
        .tools(tools)
        .build()
        .unwrap();
    let (mut client, mut rx) = OpenAI::new(req);

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
