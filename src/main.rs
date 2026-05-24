use agent::{Message, ModelClient, OpenAI, Request};

#[tokio::main]
async fn main() {
    let req = Request::empty();
    let (mut client, _) = OpenAI::new(req);
    client.add_message(Message::system("you are a man!"));
    client.add_message(Message::user_text("what is this?"));
    let _res = client.run_for_result().await;
}
