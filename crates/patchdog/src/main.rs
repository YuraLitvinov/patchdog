use rust_parsing::error::ErrorBinding;
use crate::cli::cli_patch_to_agent;
pub mod binding;
pub mod cli;

#[cfg(test)]
pub mod tests;
#[tokio::main]
/*Accepts relative path from inside folder
*/
async fn main() -> Result<(), ErrorBinding> {
    cli_patch_to_agent(true).await?;
    Ok(())
}
