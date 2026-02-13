use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    octo_code_agent::cli::run_cli().await
}
