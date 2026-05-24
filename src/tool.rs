use std::{collections::HashMap, future::Future, pin::Pin};

use crate::{
    ToolCall,
    error::{Error, Result},
};

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
    handlers: HashMap<String, Box<dyn ToolExecutor>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_executor(
        &mut self,
        name: impl Into<String>,
        executor: impl ToolExecutor + 'static,
    ) {
        self.handlers.insert(name.into(), Box::new(executor));
    }

    pub fn insert_fn<F, Fut>(&mut self, name: impl Into<String>, f: F)
    where
        F: FnMut(ToolCall) -> Fut + Send + 'static,
        Fut: Future<Output = Result<ToolOutput>> + Send + 'static,
    {
        self.insert_executor(name, ToolFn::new(f));
    }
}

impl ToolExecutor for ToolRegistry {
    fn call<'a>(&'a mut self, call: ToolCall) -> ToolFuture<'a> {
        Box::pin(async move {
            let name = call.name.clone();
            let handler = self
                .handlers
                .get_mut(&name)
                .ok_or(Error::ToolNotFound { name })?;
            handler.call(call).await
        })
    }
}
