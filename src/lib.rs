pub mod error;
mod message;
pub mod provider;
pub use message::{Message, ToolCall, UserMessageContent};
pub use provider::Provider as ModelClient;
pub use provider::{OpenAI, Provider};
use serde_json::Value;
pub struct Request {
    tools: Vec<Tool>,
    messages: Vec<Message>,
    extra: Option<Value>,
}
impl Request {
    pub fn new(tools: Vec<Tool>, messages: Vec<Message>) -> Self {
        Self {
            tools,
            messages,
            extra: None,
        }
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
        description: impl Into<Option<String>>,
        parameters: impl Into<Option<serde_json::Value>>,
        strict: Option<bool>,
    ) -> Self {
        Self::Function {
            name: name.into(),
            description: description.into(),
            parameters: parameters.into(),
            strict,
        }
    }
}
pub enum AgentState {}
pub enum AgentOutputPart {
    Content(String),
    Reasoning(String),
    Tool(Vec<ToolCall>),
}
#[derive(Debug)]
pub struct AngentOutput {
    pub contents: Vec<Message>,
}

pub struct Client<T> {
    inner: T,
}

impl<T: Provider> Provider for Client<T> {
    fn run_for_result<'a>(&'a mut self) -> provider::ProviderFuture<'a> {
        self.inner.run_for_result()
    }
}
