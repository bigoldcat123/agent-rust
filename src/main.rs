use agent::{Message, OpenAI, UserMessageContent};

#[tokio::main]
async fn main() {
    let (mut client,_) = OpenAI::new();
    client.add_message(Message::User { content: UserMessageContent::Text { content: "()".into() } });
    let res = client.await;

}
