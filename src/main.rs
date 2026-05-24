use agent::{AgentOutputPart, Message, ModelClient, OpenAI, Request, Tool};
use tokio::io::{AsyncWriteExt, stdout};

#[tokio::main]
async fn main() {
    let req = Request::empty();
    let (mut client, mut rx) = OpenAI::new(req);
    client.add_message(Message::system(r#"you are a man!"#));
    client.add_message(Message::user_text("what is math? I am the f math!"));

    tokio::spawn(async move {
       let _res = client.run_for_result().await;
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
            _ => {}
        }
    }
}
