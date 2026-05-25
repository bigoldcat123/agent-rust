use agent::{
    Client, Message, Provider, RequestBuilder, match_agent
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Serialize, Deserialize, Debug)]
pub enum Intent {
    Ticket,
    Arrange,
    Other,
}

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    match_agent!(Intent,user_input,
        Ticket => {
            println!("matched ticket {user_input:?}");
        },
        Arrange => {
            println!("matched arrange");
        },
        Other => {
            println!("matched other");
        }
    );
    let messages = vec![
        Message::system(
            r#"你需要区分用户的意图，要么是订票，要么是行程安排， 你最后会返回一个 json字符串：【"Ticket", "Arrange", "Other" 输出格式：
            {
            "answer":<your answer>
            }
            "#,
        ),
        Message::user_text("给我订一张票"),
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

    let mut agent = IntentAgent::new(Client::new(),"".into());
    let res = agent.run_for_result(req).await?;
    println!("{:?}",res);
    Ok(())
}
