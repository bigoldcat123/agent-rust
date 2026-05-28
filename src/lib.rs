pub mod agent;
pub mod error;
mod message;
pub mod provider;
pub mod step;
pub mod tool;
pub mod util;
use std::sync::Arc;

use async_openai::config::OpenAIConfig;
use derive_builder::Builder;
pub use message::{Message, ToolCall, UserMessageContent};
pub use provider::{OpenAI, Provider};
use serde_json::Value;
pub use tool::{NoopToolExecutor, ToolExecutor, ToolFn, ToolFuture, ToolOutput, ToolRegistry};

use crate::tool::SharedRegisteredTool;
#[derive(Builder)]
pub struct Request {
    #[builder(setter(strip_option,into),default)]
    tools: Option<Vec<SharedRegisteredTool>>,
    #[builder(default)]
    messages: Vec<Message>,
    #[builder(setter(into, strip_option), default)]
    extra: Option<Value>,
    #[builder(setter(into),default=format!("deepseek-v4-flash"))]
    modle: String,
}
impl Clone for Request {
    fn clone(&self) -> Self {
        Self { tools: self.tools.clone(), messages: self.messages.clone(), extra: self.extra.clone(), modle: self.modle.clone() }
    }
}
impl Request {
    pub fn get_last_message(&self) -> Option<Message> {
        self.messages.last().cloned()
    }
    pub fn get_last_user_message(&self) -> Result<UserMessageContent, crate::error::Error> {
        if let Some(Message::User { content }) = self.get_last_message() {
            Ok(content)
        } else {
            Err(crate::error::Error::MissingUserContent)
        }
    }
    pub fn messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.messages
    }
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
impl AngentOutput {
    /// (content,reasoning)
    pub fn get_last_assistant_message(&self) -> Option<(String, String)> {
        if let Some(Message::Assistant {
            content: Some(c),
            reasoning: Some(r),
            tool_calls: _,
        }) = self.contents.last()
        {
            Some((c.clone(), r.clone()))
        } else {
            None
        }
    }
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

    pub fn with_tx(mut self) -> (Self, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        self.inner.output_part_tx = Some(tx);
        (self, rx)
    }
}

impl Default for Client<OpenAI<async_openai::Client<OpenAIConfig>>> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Provider> Provider for Client<T> {
    fn run_for_result<'a>(&'a mut self, req: Request) -> provider::ProviderFuture<'a> {
        self.inner.run_for_result(req)
    }
}
