pub mod adaptor;
use async_openai::{
    config::OpenAIConfig,
    types::chat::{CreateChatCompletionRequest, FinishReason},
};
use futures::StreamExt;

use crate::{
    AgentOutputPart, AngentOutput, Message, Request, ToolCall,
    error::{Error, Result},
    tool::{NoopToolExecutor, ToolExecutor},
};

use super::{Provider, ProviderFuture};

pub struct OpenAI<C> {
    output_part_tx: Option<tokio::sync::mpsc::Sender<AgentOutputPart>>,
    req: Request,
    out_messages: Vec<Message>,
    client: C,
    tool_executor: Box<dyn ToolExecutor>,
}

impl OpenAI<async_openai::Client<OpenAIConfig>> {
    pub fn new(req: Request) -> Self{
        Self{
            output_part_tx:None,
            req,
            out_messages: vec![],
            client: async_openai::Client::new(),
            tool_executor: Box::new(NoopToolExecutor),
        }
    }

    pub fn with_tool_executor<T>(mut self, tool_executor: T) -> Self
    where
        T: ToolExecutor + 'static,
    {
        self.tool_executor = Box::new(tool_executor);
        self
    }
    pub fn with_tx(mut self) -> (Self, tokio::sync::mpsc::Receiver<AgentOutputPart>) {
        let (tx, rx) = tokio::sync::mpsc::channel(1024);
        self.output_part_tx = Some(tx);
        (self, rx)
    }

    pub(crate) fn create_chat_completion_request(&self) -> Result<CreateChatCompletionRequest> {
        let mut r: CreateChatCompletionRequest = (&self.req).try_into()?;
        let pre_messages = self.out_messages.iter().map(Into::into).collect::<Vec<_>>();
        r.messages.extend(pre_messages);
        Ok(r)
    }

    async fn send_output_part(
        output_part_tx: Option<tokio::sync::mpsc::Sender<AgentOutputPart>>,
        part: AgentOutputPart,
    ) -> Result<()> {
        if let Some(tx) = output_part_tx {
            tx.send(part).await.map_err(|_| Error::OutputClosed)?;
        }
        Ok(())
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
                            Self::send_output_part(
                                self.output_part_tx.clone(),
                                AgentOutputPart::Reasoning(r.to_string()),
                            )
                            .await?;
                        }
                        if let Some(ref c) = choice.delta.content {
                            content.push_str(c);
                            Self::send_output_part(
                                self.output_part_tx.clone(),
                                AgentOutputPart::Content(c.to_string()),
                            )
                            .await?;
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
                                    Self::send_output_part(
                                        self.output_part_tx.clone(),
                                        AgentOutputPart::Tool(tools.clone()),
                                    )
                                    .await?;
                                    let mut tool_res = Vec::with_capacity(tools.len());
                                    for tool in tools.iter().cloned() {
                                        let output = self.tool_executor.call(tool).await?;
                                        tool_res.push(Message::tool(
                                            output.tool_call_id,
                                            output.content,
                                        ));
                                    }

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
