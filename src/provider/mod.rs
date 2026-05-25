use std::{future::Future, pin::Pin};

use crate::{AngentOutput, Request, error::Result};

mod openai;

pub use openai::OpenAI;

pub type ProviderFuture<'a> = Pin<Box<dyn Future<Output = Result<AngentOutput>> + Send + 'a>>;

pub trait Provider {
    fn run_for_result<'a>(&'a mut self, req: Request) -> ProviderFuture<'a>;
}
