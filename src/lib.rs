pub mod error;
mod message;
pub mod provider;
pub mod tool;
use async_openai::config::OpenAIConfig;
use derive_builder::Builder;
pub use message::{Message, ToolCall, UserMessageContent};
pub use provider::Provider as ModelClient;
pub use provider::{OpenAI, Provider};
use serde_json::Value;
pub use tool::{NoopToolExecutor, ToolExecutor, ToolFn, ToolFuture, ToolOutput, ToolRegistry};
#[derive(Builder)]
pub struct Request {
    #[builder(default)]
    tools: Vec<Tool>,
    #[builder(default)]
    messages: Vec<Message>,
    #[builder(setter(into, strip_option), default)]
    extra: Option<Value>,
    #[builder(setter(into),default=format!("deepseek-v4-flash"))]
    modle: String,
}

#[derive(Clone)]
pub enum Tool {
    Function {
        name: String,
        description: Option<String>,
        parameters: Option<serde_json::Value>,
        strict: Option<bool>,
    },
}
impl Tool {
    pub fn new(
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

    pub fn name(&self) -> &str {
        match self {
            Self::Function { name, .. } => name,
        }
    }
}

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
impl Client<OpenAI<async_openai::Client<OpenAIConfig>>> {
    pub fn new() -> Self {
        Client {
            inner: OpenAI::new(),
        }
    }
}

impl<T: Provider> Provider for Client<T> {
    fn run_for_result<'a>(&'a mut self,req:Request) -> provider::ProviderFuture<'a> {
        self.inner.run_for_result(req)
    }
}
