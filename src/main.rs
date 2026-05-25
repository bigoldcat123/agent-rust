use agent::{
    Message, OpenAI, Provider, RequestBuilder, ToolRegistry,
    tool::{ask_user_tool, shell_tool, weather_tool},
    util::tui,
};
use serde_json::json;

#[tokio::main]
async fn main() {
    let mut tool_registry = ToolRegistry::new();
    tool_registry.insert(weather_tool());
    tool_registry.insert(shell_tool());
    tool_registry.insert(ask_user_tool());
    let messages = vec![
        Message::system(
            r#"每次都返回以josn的形式返回， {
                "mood":"your current mood",
                "answer":"your answer"
            }"#,
        ),
        Message::user_text(
            "请你使用 ask_user tool问我几个问题，然后总结出我是一个什么人。最后以json的形式回答。",
        ),
    ];
    let req = RequestBuilder::default()
        .modle("deepseek-v4-pro")
        .messages(messages)
        .tools(tool_registry.tools())
        .extra(json!({
            "response_format":{
                "type":"json_object"
            }
        }))
        .build()
        .unwrap();
    let (mut client, rx) = OpenAI::new().with_tool_executor(tool_registry).with_tx();
    tokio::spawn(async move {
        let _res = client.run_for_result(req).await;
        println!("\n{:?}", _res);
    });
    tui(rx).await;
}
