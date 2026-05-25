use tokio::{
    io::{AsyncWriteExt, stdout},
    sync::mpsc::Receiver,
};

use crate::AgentOutputPart;

pub async fn tui(mut rx: Receiver<AgentOutputPart>) -> crate::error::Result<()> {
    let mut is_reasoning = false;
    let mut stdout = stdout();
    while let Some(msg) = rx.recv().await {
        match msg {
            AgentOutputPart::Content(text) => {
                if is_reasoning {
                    is_reasoning = false;
                    println!("\n -> answer 🤓")
                }
                stdout
                    .write_all(text.as_bytes())
                    .await
                    .map_err(|source| crate::error::Error::OutputWrite { source })?;
                stdout
                    .flush()
                    .await
                    .map_err(|source| crate::error::Error::OutputWrite { source })?;
                // print!("{}", text)
            }
            AgentOutputPart::Reasoning(r) => {
                if !is_reasoning {
                    println!("\n -> reasoning 🤔");
                    is_reasoning = true
                }
                stdout
                    .write_all(r.as_bytes())
                    .await
                    .map_err(|source| crate::error::Error::OutputWrite { source })?;
                stdout
                    .flush()
                    .await
                    .map_err(|source| crate::error::Error::OutputWrite { source })?;
            }
            AgentOutputPart::Tool(tools) => {
                for t in tools {
                    println!("\ntool call -> {}", t.name);
                }
            }
        }
    }
    Ok(())
}
