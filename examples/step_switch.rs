use agent::{
    Client, Message, Provider, Request, RequestBuilder,
    provider::ProviderFuture,
    step::{JsonOutput, Layer, OpenAiProvider, Runner, WorkFlowBuilder},
    tool::ask_user_tool,
};

pub struct Switch<Yes, No> {
    yes_branch: Yes,
    no_branch: No,
}

impl<Yes, No> Switch<Yes, No> {
    pub fn new(yes: Yes, no: No) -> Self {
        Self {
            yes_branch: yes,
            no_branch: no,
        }
    }
}

impl<Yes, No> Runner for Switch<Yes, No>
where
    Yes: Runner + Send,
    No: Runner + Send,
{
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a> {
        Box::pin(async move {
            match raw_input.as_str() {
                "Yes" => self.yes_branch.run(raw_input).await,
                _ => self.no_branch.run(raw_input).await,
            }
        })
    }
}

pub struct InfoCollector<N> {
    inner: OpenAiProvider,
    next: N,
    request: Request,
}

impl<N: Runner + Send> Runner for InfoCollector<N> {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a> {
        Box::pin(async move {
            let mut req = self.request.clone();
            req.messages_mut().push(Message::user_text(raw_input));
            let req = self.inner.run_for_result(req);
            let raw_input = req.await?.get_last_assistant_message().unwrap().0;
            self.next.run(raw_input).await
        })
    }
}

pub struct InfoCollectorLayer {
    req: Request,
}

impl InfoCollectorLayer {
    pub fn new() -> Self {
        let req = RequestBuilder::default()
            .messages(vec![Message::system("你是一个信息收集者，你会收集用户的名字 年龄 爱好，如果用户没有输入这三个信息， 请你询问用户， 并输出这三个信息，")
            ])
            .tools(vec![ask_user_tool()])
            .build()
            .unwrap();
        Self { req }
    }
}

impl Default for InfoCollectorLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<I> Layer<I> for InfoCollectorLayer {
    type Out = InfoCollector<I>;

    fn layer(self, next: I) -> Self::Out {
        InfoCollector::new(Client::new(), next, self.req)
    }
}

impl<N> InfoCollector<N> {
    pub fn new(c: OpenAiProvider, n: N, req: Request) -> Self {
        Self {
            inner: c,
            next: n,
            request: req,
        }
    }
}

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let yes = WorkFlowBuilder::new().output(JsonOutput::new(Client::new()));
    let no = WorkFlowBuilder::new()
        .layer(InfoCollectorLayer::new())
        .output(JsonOutput::new(Client::new()));
    let mut out = WorkFlowBuilder::new()
        .layer(InfoCollectorLayer::new())
        .switch(Switch::new(yes, no));
    let res = out.run("我的名字是大佬猫".into()).await?;
    println!("{res:?}");
    Ok(())
}
