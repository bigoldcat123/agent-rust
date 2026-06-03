use agent::{AngentOutput, match_agent};

match_agent! {IntentMatcher,ipt,
Ticket => {
    run_ticket(ipt).await
}
Schedule => {
    run_schedule(ipt).await
}}
match_agent! {YesOrNoMatcher,ipt,
Yes => {
    run_yes(ipt).await
}
No => {
    run_no(ipt).await
}}

async fn run_yes(_msg: String) -> agent::error::Result<AngentOutput> {
    unimplemented!()
}
async fn run_no(_msg: String) -> agent::error::Result<AngentOutput> {
    unimplemented!()
}
async fn run_ticket(_msg: String) -> agent::error::Result<AngentOutput> {
    unimplemented!()
}
async fn run_schedule(_msg: String) -> agent::error::Result<AngentOutput> {
    unimplemented!()
}

#[tokio::main]
async fn main() -> agent::error::Result<()> {
    Ok(())
}
