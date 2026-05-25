use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    OpenAI(#[from] async_openai::error::OpenAIError),

    #[error("provider output receiver has been dropped")]
    OutputClosed,

    #[error("failed to write agent output: {source}")]
    OutputWrite {
        #[source]
        source: std::io::Error,
    },

    #[error("provider response did not include any choices")]
    EmptyChoices,

    #[error("tool call at index {index} is missing {field}")]
    MissingToolCallField { index: usize, field: &'static str },

    #[error("tool call delta at index {index} has no accumulated tool call")]
    ToolCallDeltaOutOfBounds { index: usize },

    #[error("tool `{name}` not found")]
    ToolNotFound { name: String },

    #[error("tool `{name}` failed: {message}")]
    ToolFailed { name: String, message: String },

    #[error("invalid tool arguments for `{name}`: {source}")]
    InvalidToolArguments {
        name: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("assistant response did not include content")]
    MissingAssistantContent,
    #[error("user response did not include content")]
    MissingUserContent,

    #[error("request did not include any messages")]
    MissingMessage,

    #[error("invalid assistant response: {source}")]
    InvalidAssistantResponse {
        #[source]
        source: serde_json::Error,
    },

    #[error("failed to build request: {message}")]
    RequestBuild { message: String },
}
