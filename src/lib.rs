use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
    task::{Context, Poll, ready},
};

use pin_project_lite::pin_project;

pub enum UserMessageContent {
    Text{content:String}
}
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub enum Message {
    User { content: UserMessageContent },
    System {
        content: String,
    },
    Assistant {
        content: Option<String>,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}
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

pub struct OpenAI {
    output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    messages: Vec<Message>,
    tools: Vec<Tool>,
}
impl OpenAI {
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
                    output_part_tx: tx,
                    messages,
                    tools,
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
impl ModelClient for OpenAI {
    type Error = String;
    fn poll_run_for_result(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<AngentOutput, Self::Error>> {
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
