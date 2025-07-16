//https://interoperable-europe.ec.europa.eu/sites/default/files/custom-page/attachment/2020-03/EUPL-1.2%20EN.txt
use crate::binding::ErrorBinding;
use crate::cli::cli_patch_mode;
pub mod binding;
pub mod cli;
#[cfg(test)]
pub mod tests;
#[tokio::main]
async fn main() -> Result<(), ErrorBinding> {
    cli_patch_mode()?;
    Ok(())
}
