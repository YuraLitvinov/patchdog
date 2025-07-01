//use dotenv::dotenv;
use patchdog::receive_context;
use std::env;
use std::path::Path;
#[tokio::main]

async fn main() {
    //dotenv().ok();
    let args: Vec<String> = env::args().collect();
    let file_path = Path::new(&args[1]);
    let receive = receive_context(file_path);
    println!("{}", receive.join("\n"));
}
