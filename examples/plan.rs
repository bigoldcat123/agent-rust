use agent::{
    AgentOutputPart, Client, Message, OpenAI, Provider, Request, RequestBuilder, ToolRegistry,
    agent::planner::WithPlanner,
    tool::{ask_user_tool, shell_tool},
    util::tui,
};
use tokio::io::{AsyncWriteExt, stdout};

#[tokio::main]
async fn main() {
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
        .tools(tool_registory.tools())
        .build()
        .unwrap();
    let (agent, rx) = Client::new().with_tool_executor(tool_registory).with_tx();
    let mut agent = agent.with_planner();
    tokio::spawn(async move {
        let res = agent.run_for_result(req).await.unwrap();
        println!("{:?}", res);
    });
    tui(rx).await;
}
