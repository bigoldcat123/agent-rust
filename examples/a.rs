use agent::{Client, Message, Provider, RequestBuilder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Serialize, Deserialize, Debug)]
pub enum Intent {
    Ticket,
    Arrange,
    Other,
}

#[tokio::main]
async fn main() {
    let messages = vec![
        Message::system(
            r#"你需要区分用户的意图，要么是订票，要么是行程安排， 你最后会返回一个 json字符串：【"Ticket", "Arrange", "Other" 输出格式：
            {
            "user_intent":<your answer>
            }
            "#,
        ),
        Message::user_text("给我订一张去东京的机票"),
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
        .unwrap();
    let mut client = Client::new();
    let res = client.run_for_result(req).await.unwrap();
    match res.contents.last() {
        Some(Message::Assistant {
            content,
            reasoning: _,
            tool_calls: _,
        }) => {
            let json = content
                .as_ref()
                .map(|x| serde_json::from_str::<Value>(x))
                .unwrap()
                .unwrap();
            println!("{json}");
            let res = json.get("user_intent").unwrap().to_string();
            println!("{res}");
            let intent: Intent = serde_json::from_str(&res).unwrap();
            match intent {
                Intent::Arrange => {
                    println!("arrange")
                }
                Intent::Other => {
                    println!("other")
                }
                Intent::Ticket => {
                    println!("ticket")
                }
            }
        }
        _ => (),
    }
}

#[derive(Deserialize)]
enum X {
    Yes,
    No
}
#[derive(Deserialize)]
struct R {
    answer:X
}

struct MatchAgent<T,E> {
    inner: Client<T>,
    e:E
}
impl <T:Provider + Send> Provider for MatchAgent<T,R> {
    fn run_for_result<'a>(&'a mut self, req: agent::Request) -> agent::provider::ProviderFuture<'a> {
        Box::pin(async move {
            let res = self.inner.run_for_result(req).await?;
            if let Some(Message::Assistant { content:Some(content), reasoning:_, tool_calls :_}) = res.contents.last() {
                let r = serde_json::from_str::<R>(content).expect("msg");
                match r.answer {
                    X::Yes => {
                        unimplemented!()
                    }
                    X::No => {
                        unimplemented!()
                    }
                }
            }
            Err(agent::error::Error::OutputClosed)
        })
    }
}
