use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};

use crate::{
    Tool, ToolCall,
    error::{Error, Result},
};

mod ask_user;
mod shell;
mod weather;

pub use ask_user::ask_user_tool;
pub use shell::shell_tool;
pub use weather::weather_tool;

#[derive(Debug, Clone)]
pub struct ToolOutput {
    pub tool_call_id: String,
    pub content: String,
}

impl ToolOutput {
    pub fn new(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
        }
    }
}

pub type ToolFuture<'a> = Pin<Box<dyn Future<Output = Result<ToolOutput>> + Send + 'a>>;

pub trait ToolExecutor: Send {
    fn call<'a>(&'a self, call: ToolCall) -> ToolFuture<'a>;
}
pub trait ToolProvider: Send {
    fn tools(&self) -> Vec<Tool>;
}
pub trait ToolProviderAndExecutor: ToolExecutor + ToolProvider {}
impl<T: ToolExecutor + ToolProvider> ToolProviderAndExecutor for T {}

impl<T> ToolExecutor for Box<T>
where
    T: ToolExecutor + ?Sized,
{
    fn call<'a>(&'a self, call: ToolCall) -> ToolFuture<'a> {
        (**self).call(call)
    }
}

pub struct NoopToolExecutor;

impl ToolExecutor for NoopToolExecutor {
    fn call<'a>(&'a self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin(async move { Ok(ToolOutput::new(call.id, "")) })
    }
}
impl ToolProvider for NoopToolExecutor {
    fn tools(&self) -> Vec<Tool> {
        vec![]
    }
}

pub struct ToolFn<F> {
    f: F,
}

impl<F> ToolFn<F> {
    pub fn new(f: F) -> Self {
        Self { f }
    }
}

impl<F, Fut> ToolExecutor for ToolFn<F>
where
    F: Fn(ToolCall) -> Fut + Send,
    Fut: Future<Output = Result<ToolOutput>> + Send + 'static,
{
    fn call<'a>(&'a self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin((self.f)(call))
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, SharedRegisteredTool>,
}
pub type SharedRegisteredTool = Arc<RegisteredTool>;
pub struct RegisteredTool {
    pub(crate) tool: Tool,
    executor: Box<dyn ToolExecutor + Send + Sync + 'static>,
}
impl RegisteredTool {
    pub fn new(
        tool: Tool,
        executor: impl ToolExecutor + Send + Sync + 'static,
    ) -> SharedRegisteredTool {
        Arc::new(Self {
            tool,
            executor: Box::new(executor),
        })
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, tool: SharedRegisteredTool) {
        self.tools.insert(tool.tool.name().to_string(), tool);
    }

    pub fn tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name).map(|entry| &entry.tool)
    }

    pub fn tools(&self) -> Vec<Tool> {
        self.tools
            .values()
            .map(|entry| entry.tool.clone())
            .collect()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl ToolProvider for ToolRegistry {
    fn tools(&self) -> Vec<Tool> {
        self.tools()
    }
}

impl ToolExecutor for ToolRegistry {
    fn call<'a>(&'a self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin(async move {
            let name = call.name.clone();
            let handler = self.tools.get(&name).ok_or(Error::ToolNotFound { name })?;
            handler.executor.call(call).await
        })
    }
}
