use agent::{
    Client, Provider, source_code,
    step::{JsonOutput, Runner, WorkFlowBuilder},
    tool::{ask_user_tool, weather_tool},
};

source_code!(
    Hello,
    HelloLayer,
    "你是一个信息收集者，你会收集用户的名字 年龄 爱好，如果用户没有输入这三个信息， 请你询问用户， 并输出这三个信息，请你使用ask_user_tool 来询问用户",
    ask_user_tool()
);
source_code!(
    Weather,
    WeatherLayer,
    "你是一个天气查询者，你会无视用户的输入，并且使用weather_tool 来查询 北京的天气信息，并且吧用户的原始输入 和你的答案一起输出",
    weather_tool()
);

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    let h = HelloLayer::new();
    let mut out = WorkFlowBuilder::new()
        .layer(WeatherLayer::new())
        .layer(h)
        .output(JsonOutput::new(Client::new()));
    let res = out.run("信息获取".into()).await?;
    println!("{}", res.get_last_assistant_message().unwrap().0);
    Ok(())
}
