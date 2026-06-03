use async_openai::config::OpenAIConfig;
use serde_json::json;

use crate::{Client, Message, OpenAI, Provider, Request, RequestBuilder, provider::ProviderFuture};

pub trait Runner {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a>;
}

pub type OpenAiProvider = Client<OpenAI<async_openai::Client<OpenAIConfig>>>;

pub struct JsonOutput {
    inner: OpenAiProvider,
    request: Request,
}

impl Runner for JsonOutput {
    fn run<'a>(&'a mut self, raw_input: String) -> ProviderFuture<'a> {
        println!("{:?}", raw_input);
        let mut req = self.request.clone();
        req.messages_mut().push(Message::user_text(raw_input));
        self.inner.run_for_result(req)
    }
}

impl JsonOutput {
    pub fn new(c: OpenAiProvider) -> Self {
        let req = RequestBuilder::default()
            .messages(vec![Message::system(
                "你会收到 用户的年龄 名字和爱好请你 返回json  {name,age,hobby,weather}",
            )])
            .extra(json!({
                "response_format": {
                    "type": "json_object"
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

#[macro_export]
macro_rules! source_code {
    ($name:ident,$layer_name:ident,$prompt:expr,$($tools:expr),* $(,)*) => {
        struct $name<N> {
            inner: $crate::step::OpenAiProvider,
            next: N,
            request: $crate::Request,
        }

        impl<N: $crate::step::Runner + Send> $crate::step::Runner for $name<N> {
            fn run<'a>(
                &'a mut self,
                raw_input: String,
            ) -> $crate::provider::ProviderFuture<'a> {
                // println!("{:?}", raw_input);
                Box::pin(async move {
                    let mut req = self.request.clone();
                    req.messages_mut().push($crate::Message::user_text(raw_input));
                    let req = self.inner.run_for_result(req);
                    let raw_input = req.await?.get_last_assistant_message().unwrap().0;
                    self.next.run(raw_input).await
                })
            }
        }

        struct $layer_name {
            req: $crate::Request,
        }

        impl $layer_name {
            fn new() -> Self {
                let req = $crate::RequestBuilder::default()
                    .messages(vec![$crate::Message::system($prompt)])
                    .tools(vec![$($tools,)*])
                    .build()
                    .unwrap();
                Self { req }
            }
        }

        impl<I> $crate::step::Layer<I> for $layer_name {
            type Out = $name<I>;

            fn layer(self, next: I) -> Self::Out {
                $name {
                    inner: $crate::Client::new(),
                    next,
                    request: self.req,
                }
            }
        }
    };
}

pub trait Layer<Inner> {
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

pub struct Stack<Inner, Outer> {
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

pub struct WorkFlowBuilder<L> {
    inner: L,
}

impl<L> WorkFlowBuilder<L> {
    pub fn layer<Outer>(self, layer: Outer) -> WorkFlowBuilder<Stack<Outer, L>> {
        WorkFlowBuilder {
            inner: Stack {
                inner: layer,
                outer: self.inner,
            },
        }
    }

    pub fn output<S>(self, s: S) -> L::Out
    where
        L: Layer<S>,
    {
        self.inner.layer(s)
    }

    pub fn switch<S>(self, s: S) -> L::Out
    where
        L: Layer<S>,
    {
        self.inner.layer(s)
    }
}

impl WorkFlowBuilder<Identity> {
    pub fn new() -> Self {
        Self { inner: Identity {} }
    }
}

impl Default for WorkFlowBuilder<Identity> {
    fn default() -> Self {
        Self::new()
    }
}
