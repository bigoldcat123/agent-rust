use agent::{
    AgentOutputPart, Message, OpenAI, Provider, RequestBuilder, ToolRegistry,
    tool::{ask_user_tool, shell_tool, weather_tool},
};
use tokio::io::{AsyncWriteExt, stdout};

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
        Message::system(
            r#"you are a helpful agent，每当你有不明白的地方，你总是会调用ask_user 去询问。"#,
        ),
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
