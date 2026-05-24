#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}
impl ToolCall {
    pub(crate) fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments: String::new(),
        }
    }
}

#[derive(Clone, Debug)]
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
        tool_calls: Option<Vec<ToolCall>>,
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
            tool_calls: None,
        }
    }

    pub fn assistant_with_details(
        content: Option<String>,
        reasoning: Option<String>,
        tool_calls: Option<Vec<ToolCall>>,
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
}
