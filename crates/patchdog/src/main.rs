use crate::binding::patch_data_argument;
pub mod binding;
#[cfg(test)]
pub mod tests;
#[tokio::main]
async fn main() {
    let _ = patch_data_argument();
}
