use std::{os::unix::raw, pin::Pin};

use async_openai::config::OpenAIConfig;
use serde_json::json;

use crate::{
    AngentOutput, Client, Message, OpenAI, Provider, Request, RequestBuilder, ToolRegistry,
    provider::ProviderFuture, tool::ask_user_tool,
};

trait Runner {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a>;
}

type OpenAiProvider = Client<OpenAI<async_openai::Client<OpenAIConfig>>>;
struct JsonOutput {
    inner: OpenAiProvider,
    request: Request,
}
impl Runner for JsonOutput {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a> {
        let mut req = self.request.clone();
        req.messages_mut().push(Message::user_text(raw_input));
        let req = self.inner.run_for_result(req);
        req
    }
}
impl JsonOutput {
    fn new(c: OpenAiProvider) -> Self {
        let req = RequestBuilder::default()
            .messages(vec![Message::system(
                "你会收到 用户的年龄 名字和爱好请你 返回json  {name,age,hobby}",
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

struct InfoCollector<N> {
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
struct InfoCollectorLayer {
    req: Request,
}
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
impl<I> Layer<I> for InfoCollectorLayer {
    type Out = InfoCollector<I>;
    fn layer(self, next: I) -> Self::Out {
        InfoCollector::new(Client::new(), next, self.req)
    }
}
impl<N> InfoCollector<N> {
    fn new(c: OpenAiProvider, n: N, req: Request) -> Self {
        Self {
            inner: c,
            next: n,
            request: req,
        }
    }
}
trait Layer<Inner> {
    type Out;
    fn layer(self, next: Inner) -> Self::Out;
}

pub struct Identity {}
impl<L> Layer<L> for Identity {
    type Out = L;
    fn layer(self, next: L) -> Self::Out {
        next
    }
}

struct Stack<Inner, Outer> {
    inner: Inner,
    outer: Outer,
}
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

struct LayerBuilder<L> {
    inner: L,
}
impl<L> LayerBuilder<L> {
    fn layer<Outer>(self, layer: Outer) -> LayerBuilder<Stack<L, Outer>> {
        LayerBuilder {
            inner: Stack {
                inner: self.inner,
                outer: layer,
            },
        }
    }
    fn output<S>(self, s: S) -> L::Out
    where
        L: Layer<S>,
    {
        self.inner.layer(s)
    }
}

impl LayerBuilder<Identity> {
    fn new() -> Self {
        Self { inner: Identity {} }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn test_name() {
        let mut out = LayerBuilder::new()
            .layer(InfoCollectorLayer::new())
            .output(JsonOutput::new(Client::new()));
        let res = out.run("我的名字是大佬猫".into()).await.unwrap();
        println!("{:?}", res);
    }

    // #[test]
    // fn test_name() {
    //     let out = LayerBuilder::new()
    //         .layer(IncocollectLayer::new())
    //         .match(Match::new(
    //             LayerBuilder::new()
    //                 .layer(..)
    //                 .layer(..)
    //                 .output(..),
    //             LayerBuilder::new()
    //                 .layer(..)
    //                 .layer(..)
    //                 .match(Match2::new(
    //                     LayerBuilder::new()
    //                         .layer(..)
    //                         .layer(..)
    //                         .output(..),
    //                     LayerBuilder::new()
    //                         .layer(..)
    //                         .layer(..)
    //                         .output(..)
    //                 ))
    //         ))
    //         .unwrap();
    // }
}
