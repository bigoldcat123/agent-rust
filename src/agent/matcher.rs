use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{AngentOutput, Client, Message, Provider, Request, RequestBuilder};

#[derive(Debug, Serialize, Deserialize)]
struct MatcherOutput<R> {
    raw_input: String,
    r#type: R,
}
#[derive(Debug, Serialize, Deserialize)]
enum GoodOrBad {
    Good,
    Bad,
}

pub struct MacherAgent<C> {
    inner: C,
    json_des: String,
}
impl<C> MacherAgent<C> {
    pub fn new(c: C, json_des: impl Into<String>) -> Self {
        Self {
            inner: c,
            json_des: json_des.into(),
        }
    }
}
fn assert_json_format(req: &mut Request) -> crate::error::Result<()> {
    if let Some(extra) = req.extra.as_mut() {
        extra
            .as_object_mut()
            .ok_or(crate::error::Error::InvalidRequest)?
            .insert(
                "response_format".to_string(),
                json!({
                    "type":"json_object"
                }),
            );
    } else {
        req.extra = Some(json!({
            "response_format":{
                "type":"json_object"
            }
        }));
    }
    Ok(())
}

impl<C: Provider + Send> Provider for MacherAgent<C> {
    fn run_for_result<'a>(
        &'a mut self,
        mut req: crate::Request,
    ) -> crate::provider::ProviderFuture<'a> {
        Box::pin(async move {
            assert_json_format(&mut req)?;
            let p = r#"你是一个分类器，你会根据用户的输入来判断用户是在做什么并按照一下格式的json形式输出：
                {
                    "type":"a type",
                    "raw_input":"用户的原始输入"
                }
                type 可能是:
                "#;
            let promprt = format!("{} {}", p, self.json_des);
            let messages = vec![
                Message::system(promprt),
                Message::User {
                    content: req.get_last_user_message()?,
                },
            ];
            let req = RequestBuilder::default()
                .messages(messages)
                .build()
                .map_err(|e| crate::error::Error::RequestBuild {
                    message: e.to_string(),
                })?;
            let res = self.inner.run_for_result(req).await?;
            let (content, _) = res
                .get_last_assistant_message()
                .ok_or(crate::error::Error::MissingAssistantContent)?;
            println!("{}",content);
            let res = serde_json::from_str::<MatcherOutput<GoodOrBad>>(&content).map_err(|e| {
                crate::error::Error::RequestBuild {
                    message: e.to_string(),
                }
            })?;

            let raw_input = res.raw_input;// we need this in match branch
            match res.r#type {
                GoodOrBad::Bad => {
                    println!("{}","bad");
                    Ok(AngentOutput {contents:vec![]})
                }
                GoodOrBad::Good => {
                    println!("{}","good");
                    Ok(AngentOutput {contents:vec![]})
                }
            }
        })
    }
}

#[tokio::test]
async fn feature() {
    match_agent!(StructName,ident_for_raw_input,
        Branch1 => {}
        Branch2 => {}
    );
    let mut agent = MacherAgent::new(Client::new(), "Good or Bad");
    let req = RequestBuilder::default()
        .messages(
            vec![Message::user_text("我今天考试59分！")]
        )
        .build()
        .unwrap();
    let _res = agent.run_for_result(req).await.unwrap();
}
