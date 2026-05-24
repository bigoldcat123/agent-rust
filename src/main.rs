use agent::{AgentOutputPart, Message, OpenAI, Provider, Request, Tool};
use serde_json::json;
use tokio::io::{AsyncWriteExt, stdout};

#[tokio::main]
async fn main() {
    let mut req = Request::empty();
    req.add_tool(Tool::function_with_details(
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
    ));
    req.add_message(Message::system(r#"you are a man!"#));
    req.add_message(Message::user_text("give me the weather in Beijing"));
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
