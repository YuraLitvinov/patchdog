use crate::cli::cli_patch_to_agent;
use rust_parsing::error::ErrorBinding;
use tracing_subscriber;
pub mod binding;
pub mod cli;

#[cfg(test)]
pub mod tests;
/// The main function of the program.
///
/// # Returns
///
/// A `Result` indicating whether the program executed successfully, or an `ErrorBinding` if any error occurred.
#[tokio::main]
//Accepts relative path from inside folder
async fn main() -> Result<(), ErrorBinding> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    cli_patch_to_agent().await?;
    Ok(())
}
