use std::{collections::HashMap, future::Future, pin::Pin};

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
    fn call<'a>(&'a mut self, call: ToolCall) -> ToolFuture<'a>;
}

impl<T> ToolExecutor for Box<T>
where
    T: ToolExecutor + ?Sized,
{
    fn call<'a>(&'a mut self, call: ToolCall) -> ToolFuture<'a> {
        (**self).call(call)
    }
}

pub struct NoopToolExecutor;

impl ToolExecutor for NoopToolExecutor {
    fn call<'a>(&'a mut self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin(async move { Ok(ToolOutput::new(call.id, "")) })
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
    F: FnMut(ToolCall) -> Fut + Send,
    Fut: Future<Output = Result<ToolOutput>> + Send + 'static,
{
    fn call<'a>(&'a mut self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin((self.f)(call))
    }
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, RegisteredTool>,
}

struct RegisteredTool {
    tool: Tool,
    executor: Box<dyn ToolExecutor>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_executor(&mut self, tool: Tool, executor: impl ToolExecutor + 'static) {
        self.tools.insert(
            tool.name().to_string(),
            RegisteredTool {
                tool,
                executor: Box::new(executor),
            },
        );
    }

    pub fn insert_fn<F, Fut>(&mut self, tool: Tool, f: F)
    where
        F: FnMut(ToolCall) -> Fut + Send + 'static,
        Fut: Future<Output = Result<ToolOutput>> + Send + 'static,
    {
        self.insert_executor(tool, ToolFn::new(f));
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
}

impl ToolExecutor for ToolRegistry {
    fn call<'a>(&'a mut self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin(async move {
            let name = call.name.clone();
            let handler = self
                .tools
                .get_mut(&name)
                .ok_or(Error::ToolNotFound { name })?;
            handler.executor.call(call).await
        })
    }
}
