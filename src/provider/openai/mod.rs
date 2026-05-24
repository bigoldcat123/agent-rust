pub mod adaptor;
use async_openai::{
    config::OpenAIConfig,
    types::chat::{CreateChatCompletionRequest, FinishReason},
};
use futures::StreamExt;

use crate::{
    AgentOutputPart, AngentOutput, Client, Message, Request, ToolCall,
    error::{Error, Result},
};

use super::{Provider, ProviderFuture};

pub struct OpenAI<C> {
    output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    req: Request,
    out_messages: Vec<Message>,
    client: C,
}

impl OpenAI<async_openai::Client<OpenAIConfig>> {
    pub fn new(req: Request) -> (Client<Self>, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        let (tx, rx) = tokio::sync::mpsc::channel::<AgentOutputPart>(100);
        (Self::with_output_part_tx(req, tx), rx)
    }

    pub fn with_output_part_tx(
        req: Request,
        output_part_tx: tokio::sync::mpsc::Sender<AgentOutputPart>,
    ) -> Client<Self> {
        Client {
            inner: Self {
                output_part_tx,
                client: async_openai::Client::new(),
                req,
                out_messages: vec![],
            },
        }
    }

    pub fn create_chat_completion_request(&self) -> Result<CreateChatCompletionRequest> {
        let mut r: CreateChatCompletionRequest = (&self.req).try_into()?;
        let pre_messages = self.out_messages.iter().map(Into::into).collect::<Vec<_>>();
        r.messages.extend(pre_messages);
        Ok(r)
    }
}

impl Provider for OpenAI<async_openai::Client<OpenAIConfig>> {
    fn run_for_result<'a>(&'a mut self) -> ProviderFuture<'a> {
        Box::pin(async move {
            let chat = self.client.chat();
            let req = self.create_chat_completion_request()?;
            let mut stream = chat.create_stream(&req).await?;
            let mut reasonging = String::new();
            let mut content = String::new();
            let mut tools = vec![];
            while let Some(stream) = stream.next().await {
                match stream {
                    Ok(stream) => {
                        let choice = stream.choices.first().ok_or(Error::EmptyChoices)?;
                        if let Some(r) = choice.delta.extra.get("reasoning_content")
                            && let Some(r) = r.as_str()
                        {
                            reasonging.push_str(r);
                            self.output_part_tx
                                .send(AgentOutputPart::Reasoning(r.to_string()))
                                .await
                                .map_err(|_| Error::OutputClosed)?;
                        }
                        if let Some(ref c) = choice.delta.content {
                            content.push_str(c);
                            self.output_part_tx
                                .send(AgentOutputPart::Content(c.to_string()))
                                .await
                                .map_err(|_| Error::OutputClosed)?;
                        }
                        if let Some(ref call_tools) = choice.delta.tool_calls {
                            if tools.is_empty() {
                                tools =
                                    call_tools
                                        .iter()
                                        .enumerate()
                                        .map(|(index, x)| {
                                            let id = x.id.clone().ok_or(
                                                Error::MissingToolCallField { index, field: "id" },
                                            )?;
                                            let name = x
                                                .function
                                                .as_ref()
                                                .and_then(|f| f.name.clone())
                                                .ok_or(Error::MissingToolCallField {
                                                    index,
                                                    field: "function.name",
                                                })?;
                                            Ok(ToolCall::new(id, name))
                                        })
                                        .collect::<Result<Vec<_>>>()?;
                            } else {
                                for (i, t) in call_tools.iter().enumerate() {
                                    if let Some(ref f) = t.function
                                        && let Some(ref args) = f.arguments
                                    {
                                        tools
                                            .get_mut(i)
                                            .ok_or(Error::ToolCallDeltaOutOfBounds { index: i })?
                                            .arguments
                                            .push_str(args);
                                    }
                                }
                            }
                        }
                        if let Some(finish_reason) = &choice.finish_reason {
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
                                    self.output_part_tx
                                        .send(AgentOutputPart::Tool(tools.clone()))
                                        .await
                                        .map_err(|_| Error::OutputClosed)?;
                                    // TODO call the tools
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
                    Err(e) => return Err(e.into()),
                }
            }
            Ok(AngentOutput {
                contents: self.out_messages.clone(),
            })
        })
    }
}
