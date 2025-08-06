use crate::cli::cli_patch_to_agent;
use rust_parsing::error::ErrorBinding;
pub mod binding;
pub mod cli;

#[cfg(test)]
pub mod tests;
#[tokio::main]
//Accepts relative path from inside folder
async fn main() -> Result<(), ErrorBinding> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    cli_patch_to_agent().await?;
    Ok(())
}
