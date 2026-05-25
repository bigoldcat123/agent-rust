#[macro_export]
macro_rules! match_agent {
    ($name:ident,$user_input:ident, $($variant:ident => $body:block),+ $(,)?) => {
        paste::paste! {
            #[allow(dead_code)]
            #[derive(Debug, serde::Deserialize)]
            enum [<$name Answer>] {
                $($variant),+
            }

            #[allow(dead_code)]
            #[derive(Debug, serde::Deserialize)]
            struct [<$name Response>] {
                answer: [<$name Answer>],
            }

            #[allow(dead_code)]
            struct [<$name Agent>]<T> {
                inner: agent::Client<T>,
                json_description:String
            }

            #[allow(dead_code)]
            impl<T> [<$name Agent>]<T> {
                fn new(inner: agent::Client<T>, json_description: String) -> Self {
                    Self { inner, json_description }
                }
            }

            impl<T> agent::Provider for [<$name Agent>]<T>
            where
                T: agent::Provider + Send,
            {
                fn run_for_result<'a>(
                    &'a mut self,
                    req: agent::Request,
                ) -> agent::provider::ProviderFuture<'a> {
                    Box::pin(async move {
                        let $user_input = req.get_last_user_message()?;
                        let res = self.inner.run_for_result(req).await?;
                        let content = match res.contents.last() {
                            Some(agent::Message::Assistant {
                                content: Some(content),
                                reasoning: _,
                                tool_calls: _,
                            }) => content,
                            _ => return Err(agent::error::Error::MissingAssistantContent),
                        };
                        let parsed = serde_json::from_str::<[<$name Response>]>(content)
                            .map_err(|source| agent::error::Error::InvalidAssistantResponse {
                                source,
                            })?;
                        match parsed.answer {
                            $([<$name Answer>]::$variant => {
                                $body;
                            }),+
                        }
                        Ok(res)
                    })
                }
            }
        }
    };
}
