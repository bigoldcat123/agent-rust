use async_openai::types::chat::{
    ChatCompletionMessageToolCall, ChatCompletionMessageToolCalls,
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestUserMessage, ChatCompletionRequestUserMessageContent, ChatCompletionTool,
    ChatCompletionTools, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    FunctionCall, FunctionObject,
};
use serde_json::json;

use crate::{Message, Request, Tool, ToolCall, UserMessageContent, error::Error};

impl TryFrom<&Request> for CreateChatCompletionRequest {
    type Error = Error;

    fn try_from(value: &Request) -> Result<Self, Self::Error> {
        let messages = value.messages.iter().map(Into::into).collect::<Vec<_>>();
        let mut args = CreateChatCompletionRequestArgs::default();
        args.model(value.modle.clone())
            .messages(messages)
            .stream(true);
        if let Some(tools) = value.tools.clone() {
            args.tools(
                tools
                    .iter()
                    .map(|x| &x.tool)
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(ref extra) = value.extra {
            args.extra(extra.clone());
        }

        Ok(args.build()?)
    }
}

impl From<&Tool> for ChatCompletionTools {
    fn from(value: &Tool) -> Self {
        match value {
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

impl From<&Message> for ChatCompletionRequestMessage {
    fn from(value: &Message) -> Self {
        match value {
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
                tool_calls: tool_calls
                    .as_ref()
                    .map(|x| x.iter().map(Into::into).collect()),
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

impl From<&UserMessageContent> for ChatCompletionRequestUserMessageContent {
    fn from(value: &UserMessageContent) -> Self {
        match value {
            UserMessageContent::Text { content } => content.clone().into(),
        }
    }
}

impl From<&ToolCall> for ChatCompletionMessageToolCalls {
    fn from(value: &ToolCall) -> Self {
        ChatCompletionMessageToolCalls::Function(ChatCompletionMessageToolCall {
            id: value.id.clone(),
            function: FunctionCall {
                name: value.name.clone(),
                arguments: value.arguments.clone(),
            },
        })
    }
}
