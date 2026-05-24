use std::{future::Future, pin::Pin};

use crate::{AngentOutput, error::Error};

mod openai;

pub use openai::OpenAI;

pub type ProviderFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AngentOutput, Error>> + Send + 'a>>;

pub trait Provider {
    fn run_for_result<'a>(&'a mut self) -> ProviderFuture<'a>;
}
