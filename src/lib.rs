use std::{
    ops::{Deref, DerefMut},
    pin::Pin,
};

use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionTool, ChatCompletionTools, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, FinishReason, FunctionObject,
    },
};
use futures::StreamExt;
pub mod error;
pub mod provider;
mod message;
pub use message::{Message, ToolCall, UserMessageContent};
use serde_json::Value;
pub struct Request {
    tools: Vec<Tool>,
    messages: Vec<Message>,
    extra: Option<Value>,
}
impl Request {
    pub fn new(tools: Vec<Tool>, messages: Vec<Message>) -> Self {
        Self {
            tools,
            messages,
            extra: None,
        }
    }

    pub fn empty() -> Self {
        Self::new(vec![], vec![])
    }

    pub fn with_messages(messages: Vec<Message>) -> Self {
        Self::new(vec![], messages)
    }

    pub fn with_tools(tools: Vec<Tool>) -> Self {
        Self::new(tools, vec![])
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    pub fn add_tool(&mut self, tool: Tool) {
        self.tools.push(tool);
    }
}
pub enum Tool {
    Function {
        name: String,
        description: Option<String>,
        parameters: Option<serde_json::Value>,
        strict: Option<bool>,
    },
}
impl Tool {
    pub fn function(name: impl Into<String>) -> Self {
        Self::Function {
            name: name.into(),
            description: None,
            parameters: None,
            strict: None,
        }
    }

    pub fn function_with_details(
        name: impl Into<String>,
        description: impl Into<Option<String>>,
        parameters: impl Into<Option<serde_json::Value>>,
        strict: Option<bool>,
    ) -> Self {
        Self::Function {
            name: name.into(),
            description: description.into(),
            parameters: parameters.into(),
            strict,
        }
    }

    pub fn to_chat_completion_tool(&self) -> ChatCompletionTools {
        match self {
            Self::Function {
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
pub enum AgentState {}
pub enum AgentOutputPart {
    Content(String),
    Reasoning(String),
    Tool(Vec<ToolCall>),
}
#[derive(Debug)]
pub struct AngentOutput {
    contents: Vec<Message>,
}

type ModelClientFuture<'a> =
    Pin<Box<dyn Future<Output = Result<AngentOutput, crate::error::Error>> + 'a + Send>>;
pub trait ModelClient {
    fn run_for_result<'a>(&'a mut self) -> ModelClientFuture<'a>;
}

pub struct OpenAI<C> {
    _output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    req: Request,
    out_messages: Vec<Message>,
    client: C,
}
impl OpenAI<async_openai::Client<OpenAIConfig>> {
    pub fn new<'a>(req: Request) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<AgentOutputPart>(100);
        (
            Client {
                inner: Self {
                    _output_part_tx: tx,
                    client: async_openai::Client::new(),
                    req,
                    out_messages: vec![],
                },
            },
            rx,
        )
    }

    pub fn create_chat_completion_request(
        &self,
        model: impl Into<String>,
    ) -> CreateChatCompletionRequest {
        let mut messages = self
            .req
            .messages
            .iter()
            .map(Message::to_chat_completion_request_message)
            .collect::<Vec<_>>();
        let pre_messages = self.out_messages.iter().map(Message::to_chat_completion_request_message)
            .collect::<Vec<_>>();
        messages.extend(pre_messages);
        let mut args = CreateChatCompletionRequestArgs::default();
        args.model(model.into()).messages(messages).stream(true);

        if !self.req.tools.is_empty() {
            args.tools(
                self.req
                    .tools
                    .iter()
                    .map(Tool::to_chat_completion_tool)
                    .collect::<Vec<_>>(),
            );
        }
        if let Some(ref extra) = self.req.extra {
            args.extra(extra.clone());
        }

        args.build().expect("msg")
    }
}

impl ModelClient for OpenAI<async_openai::Client<OpenAIConfig>> {
    fn run_for_result<'a>(&'a mut self) -> ModelClientFuture<'a> {
        Box::pin(async move {
            let chat = self.client.chat();
            let req = self.create_chat_completion_request("deepseek-v4-flash");
            let mut stream = chat.create_stream(&req).await.unwrap();
            let mut reasonging = String::new();
            let mut content = String::new();
            let mut tools = vec![];
            while let Some(stream) = stream.next().await {
                match stream {
                    Ok(stream) => {
                        if let Some(r) = stream.choices[0].delta.extra.get("reasoning_content")
                            && let Some(r) = r.as_str()
                        {
                            reasonging.push_str(r);
                            self._output_part_tx
                                .send(AgentOutputPart::Reasoning(r.to_string()))
                                .await
                                .unwrap();
                        }
                        if let Some(ref c) = stream.choices[0].delta.content {
                            content.push_str(c);
                            self._output_part_tx
                                .send(AgentOutputPart::Content(c.to_string()))
                                .await
                                .unwrap();
                        }
                        if let Some(ref call_tools) = stream.choices[0].delta.tool_calls {
                            if tools.is_empty() {
                                tools = call_tools
                                    .iter()
                                    .map(|x| (x.id.clone(), x.function.as_ref().unwrap().name.clone().unwrap(), String::new()))
                                    .collect();
                            } else {
                                for (i, t) in call_tools.iter().enumerate() {
                                    if let Some(ref f) = t.function
                                        && let Some(ref name) = f.name
                                    {
                                        tools[i].1.push_str(name);
                                    }
                                    if let Some(ref f) = t.function
                                        && let Some(ref args) = f.arguments
                                    {
                                        tools[i].2.push_str(args);
                                    }
                                }
                            }
                        }
                        if let Some(finish_reason) = &stream.choices[0].finish_reason {
                            match finish_reason {
                                FinishReason::Stop => {
                                    self.out_messages.push(Message::assistant_with_details(Some(content), Some(reasonging), None));
                                    break;
                                }
                                FinishReason::ToolCalls => {
                                    let tools = tools
                                        .into_iter()
                                        .map(|x| ToolCall {
                                            id: x.0.unwrap(),
                                            name: x.1,
                                            arguments: x.2,
                                        })
                                        .collect::<Vec<_>>();
                                    self._output_part_tx.send(AgentOutputPart::Tool(tools.clone())).await.unwrap();

                                    let tool_res = tools.iter().map(|x| Message::tool(x.id.clone(),"the weather is bad")).collect::<Vec<_>>();

                                    self.out_messages.push(Message::Assistant {
                                        content: if content.is_empty() {
                                            None
                                        } else {
                                            Some(content)
                                        },
                                        reasoning: if reasonging.is_empty() {
                                            None
                                        } else {
                                            Some(reasonging)
                                        },
                                        tool_calls: Some(tools),
                                    });
                                    self.out_messages.extend(tool_res);
                                    return self.run_for_result().await;
                                }
                                _ => {}
                            }
                        }
                    }
                    Err(_e) => return Err(crate::error::Error::E),
                }
            }

            Ok(AngentOutput {
                contents: self.out_messages.clone(),
            })
        })
    }
}

pub struct Client<T> {
    inner: T,
}
// impl<T: ModelClient> ModelClient for Client<T> {
//     fn run_for_result<'a>(&'a mut self) -> ModelClientFuture<'a> {
//         self.inner.run_for_result()
//     }
// }

impl<T> Deref for Client<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Client<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl<T> AsRef<T> for Client<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}
impl<T> AsMut<T> for Client<T> {
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
