use crate::{
    Message, OpenAI, Provider, Request, RequestBuilder, ToolRegistry, tool::ask_user_tool,
};

pub struct PlanAgent<C> {
    next: C,
}
impl<C> PlanAgent<C> {
    pub fn new(next: C) -> Self {
        Self { next }
    }
}
impl<C: Provider + Send> Provider for PlanAgent<C> {
    fn run_for_result<'a>(&'a mut self, mut req: Request) -> crate::provider::ProviderFuture<'a> {
        Box::pin(async move {
            let user_message = req.get_last_user_message()?;
            let mut tool_registory = ToolRegistry::new();
            let ask_user = ask_user_tool();
            tool_registory.insert_executor(ask_user.0, ask_user.1);
            let messages = vec![
                Message::assistant(
                    "你是一个计划小能手，你会根据用户的输入来做一份详细的任务计划书，同时你将会使用ask_user工具来询问用户任何不明白的问题。你不需要执行任务，你只需要做计划即可。",
                ),
                Message::User {
                    content: user_message,
                },
            ];
            let plan_req = RequestBuilder::default()
                .messages(messages)
                .build()
                .map_err(|e| crate::error::Error::RequestBuild {
                    message: e.to_string(),
                })?;
            let mut agent = OpenAI::new().with_tool_executor(tool_registory);
            let (content, _) = agent
                .run_for_result(plan_req)
                .await?
                .get_last_assistant_message()
                .ok_or_else(|| crate::error::Error::MissingAssistantContent)?;
            println!("{content}");
            req.messages_mut().pop();
            req.messages_mut().push(Message::user_text(content));
            self.next.run_for_result(req).await
        })
    }
}
pub trait WithPlanner: Sized {
    fn with_planner(self) -> PlanAgent<Self>;
}
impl<P: Provider> WithPlanner for P {
    fn with_planner(self) -> PlanAgent<Self> {
        PlanAgent { next: self }
    }
}
