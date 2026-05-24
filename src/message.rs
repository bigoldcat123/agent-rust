use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage,
    ChatCompletionRequestUserMessageContent, FunctionCall,
};

pub enum UserMessageContent {
    Text { content: String },
}
impl UserMessageContent {
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
        }
    }
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub enum Message {
    User {
        content: UserMessageContent,
    },
    System {
        content: String,
    },
    Assistant {
        content: Option<String>,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    Tool {
        tool_call_id: String,
        content: String,
    },
}
impl Message {
    pub fn user(content: UserMessageContent) -> Self {
        Self::User { content }
    }

    pub fn user_text(content: impl Into<String>) -> Self {
        Self::user(UserMessageContent::text(content))
    }

    pub fn system(content: impl Into<String>) -> Self {
        Self::System {
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self::Assistant {
            content: Some(content.into()),
            reasoning: None,
            tool_calls: vec![],
        }
    }

    pub fn assistant_with_details(
        content: Option<String>,
        reasoning: Option<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self::Assistant {
            content,
            reasoning,
            tool_calls,
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Tool {
            tool_call_id: tool_call_id.into(),
            content: content.into(),
        }
    }

    pub fn to_chat_completion_request_message(&self) -> ChatCompletionRequestMessage {
        match self {
            Self::User { content } => {
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: content.to_chat_completion_request_user_message_content(),
                    name: None,
                })
            }
            Self::System { content } => {
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(content.clone()),
                    name: None,
                })
            }
            Self::Assistant {
                content,
                reasoning: _,
                tool_calls,
            } => ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: content
                    .clone()
                    .map(ChatCompletionRequestAssistantMessageContent::Text),
                tool_calls: (!tool_calls.is_empty()).then(|| {
                    tool_calls
                        .iter()
                        .map(ToolCall::to_chat_completion_message_tool_call)
                        .collect()
                }),
                ..Default::default()
            }),
            Self::Tool {
                tool_call_id,
                content,
            } => ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                tool_call_id: tool_call_id.clone(),
                content: ChatCompletionRequestToolMessageContent::Text(content.clone()),
            }),
        }
    }
}

impl UserMessageContent {
    pub fn to_chat_completion_request_user_message_content(
        &self,
    ) -> ChatCompletionRequestUserMessageContent {
        match self {
            Self::Text { content } => {
                ChatCompletionRequestUserMessageContent::Text(content.clone())
            }
        }
    }
}

impl ToolCall {
    pub fn to_chat_completion_message_tool_call(&self) -> ChatCompletionMessageToolCalls {
        ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
            id: self.id.clone(),
            function: FunctionCall {
                name: self.name.clone(),
                arguments: self.arguments.clone(),
            },
        })
    }
}
