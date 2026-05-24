use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessage, ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage, ChatCompletionRequestSystemMessage, ChatCompletionRequestSystemMessageContent, ChatCompletionRequestToolMessage, ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage, ChatCompletionTool, ChatCompletionTools, CreateChatCompletionRequest, CreateChatCompletionRequestArgs, FunctionObject
};
use serde_json::json;

use crate::{Message, Request, Tool, ToolCall};

impl Into<CreateChatCompletionRequest> for &Request {
    fn into(self) -> CreateChatCompletionRequest {
        let messages = self
            .messages
            .iter()
            .map(Into::into)
            .collect::<Vec<_>>();
        let mut args = CreateChatCompletionRequestArgs::default();
        args.model(self.modle.clone())
            .messages(messages)
            .stream(true);

        if !self.tools.is_empty() {
            args.tools(
                self.tools
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            );
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
                    content: content.to_chat_completion_request_user_message_content(),
                    name: None,
                })
            }
            Message::System { content } => {
                ChatCompletionRequestMessage::System(ChatCompletionRequestSystemMessage {
                    content: ChatCompletionRequestSystemMessageContent::Text(content.clone()),
                    name: None,
                })
            }
            Message::Assistant {
                content,
                reasoning: r,
                tool_calls,
            } => ChatCompletionRequestMessage::Assistant(ChatCompletionRequestAssistantMessage {
                content: content
                    .clone()
                    .map(ChatCompletionRequestAssistantMessageContent::Text),
                tool_calls: (!tool_calls.is_none()).then(|| {
                    tool_calls
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(ToolCall::to_chat_completion_message_tool_call)
                        .collect()
                }),
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
                content: ChatCompletionRequestToolMessageContent::Text(content.clone()),
            }),
    }
    }
}
