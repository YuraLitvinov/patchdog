use crate::cli::cli_patch_to_agent;
use rust_parsing::error::ErrorBinding;
pub mod binding;
pub mod cli;

#[cfg(test)]
pub mod tests;
/// The main entry point for the application, marked with `#[tokio::main]` for asynchronous execution.
/// It initializes dotenv for environment variables and tracing_subscriber for logging.
/// The core logic is delegated to `cli_patch_to_agent()`.
///
/// # Returns
///
/// An `Ok(())` on successful completion of the `cli_patch_to_agent` function.
/// An `ErrorBinding` if any error occurs during environment setup or the `cli_patch_to_agent` execution.
//Accepts relative path from inside folder
fn main() -> Result<(), ErrorBinding> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    cli_patch_to_agent()?;
    Ok(())
}
