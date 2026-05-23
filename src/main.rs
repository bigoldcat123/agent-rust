use agent::{Message, OpenAI};

#[tokio::main]
async fn main() {
    let (mut client, _) = OpenAI::new();
    client.add_message(Message::user_text("()"));
    let _res = client.await;
}
