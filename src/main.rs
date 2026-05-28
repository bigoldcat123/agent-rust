use agent::{
    Message, OpenAI, Provider, RequestBuilder, agent::info_colloctor::WithInfoCollectAgent,
    util::tui,
};
use serde_json::json;

#[tokio::main]
async fn main() -> agent::error::Result<()> {
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
        .modle("deepseek-v4-flash")
        .messages(messages)
        .extra(json!({
            "response_format":{
                "type":"json_object"
            }
        }))
        .build()
        .map_err(|e| agent::error::Error::RequestBuild {
            message: e.to_string(),
        })?;
    let (client, rx) = OpenAI::new().with_tx();
    let mut client = client.with_info_collect_agent();
    tokio::spawn(async move {
        let _res = client.run_for_result(req).await;
        println!("\n{:?}", _res);
    });
    tui(rx).await
}
