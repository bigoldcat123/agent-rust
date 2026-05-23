use std::{
    ops::{Deref, DerefMut},
    pin::{Pin, pin},
    task::{Context, Poll, ready},
};

use async_openai::{config::OpenAIConfig, types::chat::{CreateChatCompletionRequest, CreateChatCompletionRequestArgs}};
use pin_project_lite::pin_project;

mod message;
pub use message::{Message, ToolCall, UserMessageContent};

pub enum Tool {}
pub enum AgentState {}
pub enum AgentOutputPart {}
pub struct AngentOutput {}

pub trait ModelClient {
    type Error;
    fn poll_run_for_result(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<AngentOutput, Self::Error>>;
}

pub struct OpenAI<C> {
    _output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    messages: Vec<Message>,
    tools: Vec<Tool>,
    client: C,
}
impl OpenAI<async_openai::ModelClient<OpenAIConfig>> {
    pub fn new() -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        Self::equipped(vec![], vec![])
    }
    pub fn with_messages(
        messages: Vec<Message>,
    ) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        Self::equipped(vec![], messages)
    }
    pub fn with_tools(
        tools: Vec<Tool>,
    ) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        Self::equipped(tools, vec![])
    }
    pub fn equipped(
        tools: Vec<Tool>,
        messages: Vec<Message>,
    ) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<AgentOutputPart>(100);
        (
            Client {
                inner: Self {
                    _output_part_tx: tx,
                    messages,
                    tools,
                    client: async_openai::ModelClient::new(),
                },
            },
            rx,
        )
    }
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }
    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }
}
impl ModelClient for OpenAI<async_openai::ModelClient<OpenAIConfig>> {
    type Error = String;
    fn poll_run_for_result(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<AngentOutput, Self::Error>> {
        let req = CreateChatCompletionRequestArgs::default().build().expect("msg");
        let chat = self.client.chat();
        let stream = chat.create_stream(&req);
        let res = ready!(pin!(stream).poll(cx));

        Poll::Ready(Ok(AngentOutput {}))
    }
}
pin_project! {
    pub struct Client<T>{
        #[pin]
        inner:T
    }
}

impl<T: ModelClient> Future for Client<T> {
    type Output = Result<AngentOutput, T::Error>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().inner.poll_run_for_result(cx)
    }
}
impl<T> Deref for Client<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Client<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> AsRef<T> for Client<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
impl<T> AsMut<T> for Client<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
