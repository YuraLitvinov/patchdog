use crate::binding::ErrorBinding;
use crate::cli::cli_search_patch;
pub mod binding;
pub mod cli;
#[cfg(test)]
pub mod tests;
#[tokio::main]
/*Accepts relative path from inside folder
*/
async fn main() -> Result<(), ErrorBinding> {
    cli_search_patch().await?;
    Ok(())
}
