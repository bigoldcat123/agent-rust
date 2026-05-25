use agent::{
    Client, Message, Provider, RequestBuilder, ToolRegistry, agent::planner::WithPlanner,
    tool::shell_tool, util::tui,
};

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let messages = vec![
        Message::system(
            "you are a helpful agent, you will finish the task by leveaging the power of all tools that you can access",
        ),
        Message::user_text("帮我写一个个人博客，并保存到 /Users/dadigua/Desktop"),
    ];
    let mut tool_registory = ToolRegistry::new();
    let shell_tool = shell_tool();
    tool_registory.insert_executor(shell_tool.0, shell_tool.1);
    let req = RequestBuilder::default()
        .messages(messages)
        .build()
        .map_err(|e| agent::error::Error::RequestBuild {
            message: e.to_string(),
        })?;
    let (agent, rx) = Client::new().with_tool_executor(tool_registory).with_tx();
    let mut agent = agent.with_planner();
    tokio::spawn(async move {
        let res = agent.run_for_result(req).await;
        println!("{:?}", res);
    });
    tui(rx).await
}
