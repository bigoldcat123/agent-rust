#[cfg(test)]
use async_openai::config::OpenAIConfig;
#[cfg(test)]
use serde_json::json;

#[cfg(test)]
use crate::{
    Client, Message, OpenAI, Provider, Request, RequestBuilder, provider::ProviderFuture,
    tool::ask_user_tool,
};

#[cfg(test)]
trait Runner {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a>;
}

#[cfg(test)]
type OpenAiProvider = Client<OpenAI<async_openai::Client<OpenAIConfig>>>;
#[cfg(test)]
struct JsonOutput {
    inner: OpenAiProvider,
    request: Request,
}
#[cfg(test)]
impl Runner for JsonOutput {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a> {
        println!("{:?}", raw_input);
        let mut req = self.request.clone();
        req.messages_mut().push(Message::user_text(raw_input));
        self.inner.run_for_result(req)
    }
}
#[cfg(test)]
impl JsonOutput {
    fn new(c: OpenAiProvider) -> Self {
        let req = RequestBuilder::default()
            .messages(vec![Message::system(
                "你会收到 用户的年龄 名字和爱好请你 返回json  {name,age,hobby,weather}",
            )])
            .extra(json!({
                "response_format":{
                    "type":"json_object"
                }
            }))
            .build()
            .unwrap();

        Self {
            inner: c,
            request: req,
        }
    }
}

#[cfg(test)]
macro_rules! source_code {
    ($name:ident,$layer_name:ident,$prompt:expr,$($tools:expr),* $(,)*) => {
        struct $name<N> {
            inner: OpenAiProvider,
            next: N,
            request: Request,
        }
        impl<N: Runner + Send> Runner for $name<N> {
            fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a> {
                println!("{:?}",raw_input);
                Box::pin(async move {
                    let mut req = self.request.clone();
                    req.messages_mut().push(Message::user_text(raw_input));
                    let req = self.inner.run_for_result(req);
                    let raw_input = req.await?.get_last_assistant_message().unwrap().0;
                    self.next.run(raw_input).await
                })
            }
        }
        struct $layer_name {
            req: Request,
        }
        impl $layer_name {
            fn new() -> Self {
                let req = RequestBuilder::default()
                    .messages(vec![Message::system($prompt)
                    ])
                    .tools(vec![$($tools,)*])
                    .build()
                    .unwrap();
                Self { req }
            }
        }
        impl<I> Layer<I> for $layer_name {
            type Out = $name<I>;
            fn layer(self, next: I) -> Self::Out {
                $name {
                    inner: Client::new(),
                    next: next,
                    request: self.req,
                }
            }
        }
    };
}

#[cfg(test)]
struct InfoCollector<N> {
    inner: OpenAiProvider,
    next: N,
    request: Request,
}
#[cfg(test)]
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
#[cfg(test)]
struct InfoCollectorLayer {
    req: Request,
}
#[cfg(test)]
impl InfoCollectorLayer {
    fn new() -> Self {
        let req = RequestBuilder::default()
            .messages(vec![Message::system("你是一个信息收集者，你会收集用户的名字 年龄 爱好，如果用户没有输入这三个信息， 请你询问用户， 并输出这三个信息，")
            ])
            .tools(vec![ask_user_tool()])
            .build()
            .unwrap();
        Self { req }
    }
}
#[cfg(test)]
impl<I> Layer<I> for InfoCollectorLayer {
    type Out = InfoCollector<I>;
    fn layer(self, next: I) -> Self::Out {
        InfoCollector::new(Client::new(), next, self.req)
    }
}
#[cfg(test)]
impl<N> InfoCollector<N> {
    fn new(c: OpenAiProvider, n: N, req: Request) -> Self {
        Self {
            inner: c,
            next: n,
            request: req,
        }
    }
}
#[cfg(test)]
trait Layer<Inner> {
    type Out;
    fn layer(self, next: Inner) -> Self::Out;
}

#[cfg(test)]
pub struct Identity {}
#[cfg(test)]
impl<L> Layer<L> for Identity {
    type Out = L;
    fn layer(self, next: L) -> Self::Out {
        next
    }
}

#[cfg(test)]
struct Stack<Inner, Outer> {
    inner: Inner,
    outer: Outer,
}
#[cfg(test)]
impl<Inner, Outer, S> Layer<S> for Stack<Inner, Outer>
where
    Inner: Layer<S>,
    Outer: Layer<Inner::Out>,
{
    type Out = Outer::Out;
    fn layer(self, next: S) -> Self::Out {
        let out = self.inner.layer(next);
        self.outer.layer(out)
    }
}

#[cfg(test)]
struct WorkFlowBuilder<L> {
    inner: L,
}
#[cfg(test)]
impl<L> WorkFlowBuilder<L> {
    fn layer<Outer>(self, layer: Outer) -> WorkFlowBuilder<Stack<Outer, L>> {
        WorkFlowBuilder {
            inner: Stack {
                inner: layer,
                outer: self.inner,
            },
        }
    }
    fn output<S>(self, s: S) -> L::Out
    where
        L: Layer<S>,
    {
        self.inner.layer(s)
    }
    fn switch<S>(self, s: S) -> L::Out
    where
        L: Layer<S>,
    {
        self.inner.layer(s)
    }
}

#[cfg(test)]
impl WorkFlowBuilder<Identity> {
    fn new() -> Self {
        Self { inner: Identity {} }
    }
}

#[cfg(test)]
struct Switch<Yes, No> {
    yes_branch: Yes,
    no_branch: No,
}
#[cfg(test)]
impl<Yes, No> Switch<Yes, No> {
    fn new(yes: Yes, no: No) -> Self {
        Self {
            yes_branch: yes,
            no_branch: no,
        }
    }
}
#[cfg(test)]
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

#[cfg(test)]
mod tests {

    use crate::tool::weather_tool;

    use super::*;
    #[tokio::test]
    async fn test_switc2h() {
        source_code!(
            Hello,
            HelloLayer,
            "你是一个信息收集者，你会收集用户的名字 年龄 爱好，如果用户没有输入这三个信息， 请你询问用户， 并输出这三个信息，请你使用ask_user_tool 来询问用户",
            ask_user_tool()
        );
        source_code!(
            Weather,
            WeatherLayer,
            "你是一个天气查询者，你会无视用户的输入，并且使用weather_tool 来查询 北京的天气信息，并且吧用户的原始输入 和你的答案一起输出",
            weather_tool()
        );
        let h = HelloLayer::new();
        let mut out = WorkFlowBuilder::new()
            .layer(WeatherLayer::new())
            .layer(h)
            .output(JsonOutput::new(Client::new()));
        let res = out.run("信息获取".into()).await.unwrap();
        println!("{}", res.get_last_assistant_message().unwrap().0)
    }

    #[tokio::test]
    async fn test_switch() {
        let mut out = WorkFlowBuilder::new()
            .layer(InfoCollectorLayer::new())
            .output(JsonOutput::new(Client::new()));
        let res = out.run("我的名字是大佬猫".into()).await.unwrap();
        println!("{:?}", res);
    }

    #[tokio::test]
    async fn test_name() {
        let yes = WorkFlowBuilder::new().output(JsonOutput::new(Client::new()));
        let no = WorkFlowBuilder::new()
            .layer(InfoCollectorLayer::new())
            .output(JsonOutput::new(Client::new()));
        let mut out = WorkFlowBuilder::new()
            .layer(InfoCollectorLayer::new())
            .switch(Switch::new(yes, no));
        let res = out.run("我的名字是大佬猫".into()).await.unwrap();
        println!("{:?}", res);
    }
}
