/*
use filesystem_parsing::parse_all_rust_items;
use filesystem_parsing::InvalidIoOperationsSnafu;
use anyhow::Context;
use std::fs;
use std::path::Path;
*/
use git_parsing::git_get;

pub mod tests;
//const PATH: &str  = "/home/yurii-sama/Desktop/patchdog/crates/filesystem_parsing/src/lib.rs";

#[tokio::main]

async fn main() {
    let _ = git_get();
}
