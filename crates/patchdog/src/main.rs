use rust_parsing::ErrorHandling;

use crate::binding::patch_data_argument;
pub mod binding;
#[cfg(test)]
pub mod tests;
#[tokio::main]
async fn main() -> Result<(), ErrorHandling> {
    let call = patch_data_argument()?;
    Ok(call)
}
