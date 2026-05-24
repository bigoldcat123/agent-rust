use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionTool, ChatCompletionTools, CreateChatCompletionRequest,
        CreateChatCompletionRequestArgs, FinishReason, FunctionObject,
    },
};
use futures::StreamExt;

use crate::{
    AgentOutputPart, AngentOutput, Client, Message, Request, Tool, ToolCall, error::Error,
};

use super::{Provider, ProviderFuture};

pub struct OpenAI<C> {
    _output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    req: Request,
    out_messages: Vec<Message>,
    client: C,
}

impl OpenAI<async_openai::Client<OpenAIConfig>> {
    pub fn new(req: Request) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
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
        let pre_messages = self
            .out_messages
            .iter()
            .map(Message::to_chat_completion_request_message)
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

impl Provider for OpenAI<async_openai::Client<OpenAIConfig>> {
    fn run_for_result<'a>(&'a mut self) -> ProviderFuture<'a> {
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
                                    .map(|x| {
                                        (
                                            x.id.clone(),
                                            x.function.as_ref().unwrap().name.clone().unwrap(),
                                            String::new(),
                                        )
                                    })
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
                                    self.out_messages.push(Message::assistant_with_details(
                                        Some(content),
                                        Some(reasonging),
                                        None,
                                    ));
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
                                    self._output_part_tx
                                        .send(AgentOutputPart::Tool(tools.clone()))
                                        .await
                                        .unwrap();

                                    let tool_res = tools
                                        .iter()
                                        .map(|x| Message::tool(x.id.clone(), "the weather is bad"))
                                        .collect::<Vec<_>>();

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
                    Err(_e) => return Err(Error::E),
                }
            }

            Ok(AngentOutput {
                contents: self.out_messages.clone(),
            })
        })
    }
}

impl Tool {
    pub(crate) fn to_chat_completion_tool(&self) -> ChatCompletionTools {
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
