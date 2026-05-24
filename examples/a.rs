use agent::{Message, OpenAI, Provider, RequestBuilder};

#[tokio::main]
async fn main() {
    let messages = vec![
        Message::system(r#"you are a helpful agent"#),
        Message::user_text("tell me who are you?"),
    ];
    let req = RequestBuilder::default()
        .modle("deepseek-v4-flash")
        .messages(messages)
        .build()
        .unwrap();
    let mut client = OpenAI::new(req);
    let res = client.run_for_result().await.unwrap();
    let res = res.contents.last().unwrap();
    println!("{:?}", res)
}
