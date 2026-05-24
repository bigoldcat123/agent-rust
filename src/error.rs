use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    OpenAI(#[from] async_openai::error::OpenAIError),

    #[error("provider output receiver has been dropped")]
    OutputClosed,

    #[error("provider response did not include any choices")]
    EmptyChoices,

    #[error("tool call at index {index} is missing {field}")]
    MissingToolCallField { index: usize, field: &'static str },

    #[error("tool call delta at index {index} has no accumulated tool call")]
    ToolCallDeltaOutOfBounds { index: usize },
}
