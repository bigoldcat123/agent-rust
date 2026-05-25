use crate::{
    Client, Message, Provider, RequestBuilder, ToolRegistry, UserMessageContent,
    tool::ask_user_tool,
};

pub struct InfoCollectAgent<C> {
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
    pub fn with_prompt(mut self, prompt: String) -> Self {
        self.prompt = prompt;
        self
    }
}
impl<C: Provider + Send> Provider for InfoCollectAgent<C> {
    fn run_for_result<'a>(
        &'a mut self,
        mut req: crate::Request,
    ) -> crate::provider::ProviderFuture<'a> {
        Box::pin(async move {
            let messages = vec![
                Message::system(&self.prompt),
                req.get_last_message()
                    .ok_or(crate::error::Error::MissingMessage)?,
            ];
            let mut tool_registry = ToolRegistry::new();
            tool_registry.insert(ask_user_tool());
            let c_req = RequestBuilder::default()
                .messages(messages)
                .build()
                .map_err(|e| crate::error::Error::RequestBuild {
                    message: e.to_string(),
                })?;
            let mut c = Client::new().with_tool_executor(tool_registry);
            let res = c.run_for_result(c_req).await?;
            let (res, _) = res
                .get_last_assistant_message()
                .ok_or(crate::error::Error::MissingAssistantContent)?;

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

pub trait WithInfoCollectAgent: Sized {
    fn with_info_collect_agent(self) -> InfoCollectAgent<Self>;
}

impl<T: Provider> WithInfoCollectAgent for T {
    fn with_info_collect_agent(self) -> InfoCollectAgent<T> {
        InfoCollectAgent::new(self)
    }
}
