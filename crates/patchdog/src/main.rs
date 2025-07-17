//Signed commit
use crate::binding::ErrorBinding;
use crate::cli::{cli_search_mode};
pub mod binding;
pub mod cli;
#[cfg(test)]
pub mod tests;
#[tokio::main]
async fn main() -> Result<(), ErrorBinding> {
    cli_search_mode().await?;
    Ok(())
}
