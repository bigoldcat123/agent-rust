use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent, ChatCompletionTool,
    ChatCompletionTools, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    FunctionCall, FunctionObject,
};
use serde_json::json;

use crate::{Message, Request, Tool, ToolCall, UserMessageContent};

impl Into<CreateChatCompletionRequest> for &Request {
    fn into(self) -> CreateChatCompletionRequest {
        let messages = self.messages.iter().map(Into::into).collect::<Vec<_>>();
        let mut args = CreateChatCompletionRequestArgs::default();
        args.model(self.modle.clone())
            .messages(messages)
            .stream(true);

        if !self.tools.is_empty() {
            args.tools(self.tools.iter().map(Into::into).collect::<Vec<_>>());
        }
        if let Some(ref extra) = self.extra {
            args.extra(extra.clone());
        }

        args.build().expect("msg")
    }
}

impl Into<ChatCompletionTools> for &Tool {
    fn into(self) -> ChatCompletionTools {
        match self {
            Tool::Function {
                name,
                description,
                parameters,
                strict,
            } => ChatCompletionTools::Function(ChatCompletionTool {
                function: FunctionObject {
                    name: name.clone(),
                    description: description.clone(),
                    parameters: parameters.clone(),
                    strict: *strict,
                },
            }),
        }
    }
}

impl Into<ChatCompletionRequestMessage> for &Message {
    fn into(self) -> ChatCompletionRequestMessage {
        match self {
            Message::User { content } => {
                ChatCompletionRequestMessage::User(ChatCompletionRequestUserMessage {
                    content: content.into(),
                    name: None,
                })
            }
            Message::System { content } => {
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: content.clone().into(),
                    name: None,
                })
            }
            Message::Assistant {
                content,
                reasoning: r,
                tool_calls,
            } => ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: content.clone().map(Into::into),
                tool_calls: tool_calls.as_ref().map(|x| x.iter().map(Into::into).collect()),
                extra: r.clone().map(|x| {
                    json!({
                        "reasoning_content":x
                    })
                }),
                ..Default::default()
            }),
            Message::Tool {
                tool_call_id,
                content,
            } => ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                tool_call_id: tool_call_id.clone(),
                content: content.clone().into(),
            }),
        }
    }
}

impl Into<ChatCompletionRequestUserMessageContent> for &UserMessageContent {
    fn into(self) -> ChatCompletionRequestUserMessageContent {
        match self {
            UserMessageContent::Text { content } => content.clone().into(),
        }
    }
}

impl Into<ChatCompletionMessageToolCalls> for &ToolCall {
    fn into(self) -> ChatCompletionMessageToolCalls {
        ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
            id: self.id.clone(),
            function: FunctionCall {
                name: self.name.clone(),
                arguments: self.arguments.clone(),
            },
        })
    }
}
