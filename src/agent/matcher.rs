use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{AngentOutput, Client, Message, Provider, Request, RequestBuilder};

pub trait MatchAgentBranchResult {
    fn into_match_agent_result(self) -> crate::error::Result<AngentOutput>;
}

impl MatchAgentBranchResult for () {
    fn into_match_agent_result(self) -> crate::error::Result<AngentOutput> {
        Ok(AngentOutput { contents: vec![] })
    }
}

impl MatchAgentBranchResult for AngentOutput {
    fn into_match_agent_result(self) -> crate::error::Result<AngentOutput> {
        Ok(self)
    }
}

impl MatchAgentBranchResult for crate::error::Result<AngentOutput> {
    fn into_match_agent_result(self) -> crate::error::Result<AngentOutput> {
        self
    }
}

#[macro_export]
macro_rules! match_agent {
    (
        $vis:vis $agent_name:ident, $raw_input:ident,
        $($branch:ident => $body:block)+ $(,)?
    ) => {
        #[allow(dead_code)]
        $vis struct $agent_name<C> {
            inner: C,
            json_des: String,
        }

        #[allow(dead_code)]
        impl<C> $agent_name<C> {
            $vis fn new(c: C, json_des: impl Into<String>) -> Self {
                Self {
                    inner: c,
                    json_des: json_des.into(),
                }
            }

            $vis fn with_default_description(c: C) -> Self {
                Self::new(c, vec![$(stringify!($branch)),+].join(" or "))
            }
        }

        impl<C: $crate::Provider + Send> $crate::Provider for $agent_name<C> {
            fn run_for_result<'a>(
                &'a mut self,
                req: $crate::Request,
            ) -> $crate::provider::ProviderFuture<'a> {
                Box::pin(async move {
                    #[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
                    struct MatchAgentOutput {
                        raw_input: String,
                        r#type: MatchAgentType,
                    }

                    #[derive(Debug, ::serde::Serialize, ::serde::Deserialize)]
                    enum MatchAgentType {
                        $($branch),+
                    }

                    let prompt = format!(
                        "{} {}",
                        r#"你是一个分类器，你会根据用户的输入来判断用户是在做什么并按照以下格式的 json 形式输出：
                {
                    "type":"a type",
                    "raw_input":"用户的原始输入"
                }
                type 可能是:
                "#,
                        self.json_des,
                    );
                    let messages = vec![
                        $crate::Message::system(prompt),
                        $crate::Message::User {
                            content: req.get_last_user_message()?,
                        },
                    ];
                    let req = $crate::RequestBuilder::default()
                        .messages(messages)
                        .extra(::serde_json::json!({
                            "response_format": {
                                "type": "json_object"
                            }
                        }))
                        .build()
                        .map_err(|e| $crate::error::Error::RequestBuild {
                            message: e.to_string(),
                        })?;
                    let res = self.inner.run_for_result(req).await?;
                    let (content, _) = res
                        .get_last_assistant_message()
                        .ok_or($crate::error::Error::MissingAssistantContent)?;
                    let res = ::serde_json::from_str::<MatchAgentOutput>(&content).map_err(|e| {
                        $crate::error::Error::RequestBuild {
                            message: e.to_string(),
                        }
                    })?;
                    let $raw_input = res.raw_input;
                    match res.r#type {
                        $(
                            MatchAgentType::$branch => {
                                $crate::agent::matcher::MatchAgentBranchResult::into_match_agent_result($body)
                            }
                        ),+
                    }
                })
            }
        }
    };
}

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
            println!("{}", content);
            let res = serde_json::from_str::<MatcherOutput<GoodOrBad>>(&content).map_err(|e| {
                crate::error::Error::RequestBuild {
                    message: e.to_string(),
                }
            })?;

            let _raw_input = res.raw_input; // we need this in match branch
            match res.r#type {
                GoodOrBad::Bad => {
                    println!("{}", "bad");
                    Ok(AngentOutput { contents: vec![] })
                }
                GoodOrBad::Good => {
                    println!("{}", "good");
                    Ok(AngentOutput { contents: vec![] })
                }
            }
        })
    }
}

#[tokio::test]
async fn feature() {
    match_agent!(StructName,ident_for_raw_input,
        Good => {
            let mut a = Client::new();
            let req = RequestBuilder::default()
                .messages(
                    vec![
                        Message::system("用户现在特别高兴， 请你鼓励他一下， 并且制定一下快乐玩耍的计划。")
                        ,Message::user_text(ident_for_raw_input)]
                ).build().unwrap();
            a.run_for_result(req).await
        }
        Bad => {
            let mut a = Client::new();
            let req = RequestBuilder::default()
                .messages(
                    vec![
                        Message::system("用户现在特别难受， 请你阴阳一下他，并制定下一步的计划。")
                        ,Message::user_text(ident_for_raw_input)]
                ).build().unwrap();
            a.run_for_result(req).await
        }
    );
    let mut agent = StructName::new(crate::Client::new(), "Good or Bad");
    let req = RequestBuilder::default()
        .messages(vec![Message::user_text("我今天考试59分！")])
        .build()
        .unwrap();
    let _res = agent.run_for_result(req).await.unwrap();
    println!("{:?}",_res);
}
