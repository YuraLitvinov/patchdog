use crate::cli::cli_patch_to_agent;
use rust_parsing::error::ErrorBinding;
pub mod binding;
pub mod cli;

#[cfg(test)]
pub mod tests;
/// The asynchronous main entry point for the application. This function initializes environment variables using `dotenv` and sets up `tracing_subscriber` for logging.
/// It then calls `cli_patch_to_agent()` to execute the core logic of processing a Git patch and interacting with an AI agent.
///
/// # Returns
///
/// An `Ok(())` if the application runs to completion without errors, or an `ErrorBinding` if any part of the process fails.
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
