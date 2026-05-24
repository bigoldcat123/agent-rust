use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionTool, ChatCompletionTools, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, FunctionObject,
    },
};
use futures::StreamExt;
pub mod error;

mod message;
pub use message::{Message, ToolCall, UserMessageContent};
pub struct Request {
    tools: Vec<Tool>,
    messages: Vec<Message>,
}
impl Request {
    pub fn new(tools: Vec<Tool>, messages: Vec<Message>) -> Self {
        Self { tools, messages }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![])
    }

    pub fn with_messages(messages: Vec<Message>) -> Self {
        Self::new(vec![], messages)
    }

    pub fn with_tools(tools: Vec<Tool>) -> Self {
        Self::new(tools, vec![])
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }
}
pub enum Tool {
    Function {
        name: String,
        description: Option<String>,
        parameters: Option<serde_json::Value>,
        strict: Option<bool>,
    },
}
impl Tool {
    pub fn function(name: impl Into<String>) -> Self {
        Self::Function {
            name: name.into(),
            description: None,
            parameters: None,
            strict: None,
        }
    }

    pub fn function_with_details(
        name: impl Into<String>,
        description: Option<String>,
        parameters: Option<serde_json::Value>,
        strict: Option<bool>,
    ) -> Self {
        Self::Function {
            name: name.into(),
            description,
            parameters,
            strict,
        }
    }

    pub fn to_chat_completion_tool(&self) -> ChatCompletionTools {
        match self {
            Self::Function {
                name,
                description,
                parameters,
                strict,
            } => ChatCompletionTools::Function(ChatCompletionTool {
                function: FunctionObject {
                    name: name.clone(),
                    description: description.clone(),
                    parameters: parameters.clone(),
                    strict: *strict,
                },
            }),
        }
    }
}
pub enum AgentState {}
pub enum AgentOutputPart {
    Text(String),
    Reasoning(String),
    Tool(ToolCall)
}
pub struct AngentOutput {}

type ModelClientFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AngentOutput, crate::error::Error>> + 'a + Send>>;
pub trait ModelClient {
    fn run_for_result<'a>(&'a mut self) -> ModelClientFuture<'a>;
}

pub struct OpenAI<C> {
    _output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    req: Request,
    client: C,
}
impl OpenAI<async_openai::Client<OpenAIConfig>> {
    pub fn new<'a>(req: Request) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<AgentOutputPart>(100);
        (
            Client {
                inner: Self {
                    _output_part_tx: tx,
                    client: async_openai::Client::new(),
                    req,
                },
            },
            rx,
        )
    }
    pub fn add_message(&mut self, message: Message) {
        self.req.messages.push(message);
    }
    pub fn add_tool(&mut self, tool: Tool) {
        self.req.tools.push(tool);
    }

    pub fn create_chat_completion_request(
        &self,
        model: impl Into<String>,
    ) -> CreateChatCompletionRequest {
        let messages = self
            .req
            .messages
            .iter()
            .map(Message::to_chat_completion_request_message)
            .collect::<Vec<_>>();

        let mut args = CreateChatCompletionRequestArgs::default();
        args.model(model.into()).messages(messages).stream(true);

        if !self.req.tools.is_empty() {
            args.tools(
                self.req
                    .tools
                    .iter()
                    .map(Tool::to_chat_completion_tool)
                    .collect::<Vec<_>>(),
            );
        }

        args.build().expect("msg")
    }
}

impl ModelClient for OpenAI<async_openai::Client<OpenAIConfig>> {
    fn run_for_result<'a>(&'a mut self) -> ModelClientFuture<'a> {
        Box::pin(async move {
            let chat = self.client.chat();
            let req = self.create_chat_completion_request("deepseek-v4-flash");
            let mut stream = chat.create_stream(&req).await.unwrap();
            while let Some(stream) = stream.next().await {
                match stream {
                    Ok(stream) => {
                        println!("{:?}", stream.choices[0].delta.content);
                        println!("{:?}", stream.choices[0].delta.extra);
                    }
                    Err(_e) => return Err(crate::error::Error::E),
                }
            }

            Ok(AngentOutput {})
        })
    }
}

pub struct Client<T> {
    inner: T,
}
// impl<T: ModelClient> ModelClient for Client<T> {
//     fn run_for_result<'a>(&'a mut self) -> ModelClientFuture<'a> {
//         self.inner.run_for_result()
//     }
// }

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
