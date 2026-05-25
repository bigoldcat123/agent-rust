use agent::{
    Client, Message, OpenAI, Provider, RequestBuilder, ToolRegistry, UserMessageContent,
    tool::ask_user_tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

macro_rules! match_agent {
    ($name:ident,$user_input:ident, $($variant:ident => $body:block),+ $(,)?) => {
        paste::paste! {
            #[allow(dead_code)]
            #[derive(Debug, serde::Deserialize)]
            enum [<$name Answer>] {
                $($variant),+
            }

            #[allow(dead_code)]
            #[derive(Debug, serde::Deserialize)]
            struct [<$name Response>] {
                answer: [<$name Answer>],
            }

            #[allow(dead_code)]
            struct [<$name Agent>]<T> {
                inner: agent::Client<T>,
            }

            #[allow(dead_code)]
            impl<T> [<$name Agent>]<T> {
                fn new(inner: agent::Client<T>) -> Self {
                    Self { inner }
                }
            }

            impl<T> agent::Provider for [<$name Agent>]<T>
            where
                T: agent::Provider + Send,
            {
                fn run_for_result<'a>(
                    &'a mut self,
                    req: agent::Request,
                ) -> agent::provider::ProviderFuture<'a> {
                    Box::pin(async move {
                        let $user_input = if let Some(message) = req.get_last_message() {
                            message.clone()
                        }else {
                            return Err(agent::error::Error::MissingAssistantContent)
                        };
                        let res = self.inner.run_for_result(req).await?;
                        let content = match res.contents.last() {
                            Some(agent::Message::Assistant {
                                content: Some(content),
                                reasoning: _,
                                tool_calls: _,
                            }) => content,
                            _ => return Err(agent::error::Error::MissingAssistantContent),
                        };
                        let parsed = serde_json::from_str::<[<$name Response>]>(content)
                            .map_err(|source| agent::error::Error::InvalidAssistantResponse {
                                source,
                            })?;
                        match parsed.answer {
                            $([<$name Answer>]::$variant => {
                                $body;
                            }),+
                        }
                        Ok(res)
                    })
                }
            }
        }
    };
}
struct InfoCollectAgent<C> {
    next: C,
    prompt: String,
}
impl<C> InfoCollectAgent<C> {
    pub fn new(agent: C) -> Self {
        let mut prompt = String::new();
        prompt.push_str("你是一个信息收集者，你只管收集信息，最后的回答必须要包括用户的原始问题， 以及你收集到的所有信息，尽量多的使用ask_user 去获取不确定的信息。
            最后输出格式：
            用户原始问题：...
            已知的信息:...");
        InfoCollectAgent {
            next: agent,
            prompt,
        }
    }
    #[allow(dead_code)]
    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.prompt = prompt;
        self
    }
}
impl<C: Provider + Send> Provider for InfoCollectAgent<C> {
    fn run_for_result<'a>(
        &'a mut self,
        mut req: agent::Request,
    ) -> agent::provider::ProviderFuture<'a> {
        Box::pin(async move {
            let messages = vec![
                Message::system(&self.prompt),
                req.get_last_message()
                    .ok_or(agent::error::Error::MissingMessage)?,
            ];
            let mut tool_registry = ToolRegistry::new();
            let (tool, tool_executor) = ask_user_tool();
            tool_registry.insert_executor(tool, tool_executor);
            let c_req = RequestBuilder::default()
                .messages(messages)
                .tools(tool_registry.tools())
                .build()
                .map_err(|e| agent::error::Error::RequestBuild {
                    message: e.to_string(),
                })?;
            let mut c = OpenAI::new().with_tool_executor(tool_registry);
            let res = c.run_for_result(c_req).await?;
            let res = if let Some(Message::Assistant {
                content: Some(content),
                reasoning: _,
                tool_calls: _,
            }) = res.contents.last()
            {
                content.clone()
            } else {
                return Err(agent::error::Error::MissingAssistantContent);
            };
            if let Some(Message::User {
                content: UserMessageContent::Text { content },
            }) = req.messages_mut().last_mut()
            {
                *content = res;
            }

            self.next.run_for_result(req).await
        })
    }
}

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

    let agent = IntentAgent::new(Client::new());
    let mut agent = InfoCollectAgent::new(agent);
    let res = agent.run_for_result(req).await?;
    println!("{:?}", res.contents.last().cloned());
    Ok(())
}
