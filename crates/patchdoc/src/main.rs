use dotenv::dotenv;
use patchdoc::receive_context;
use std::path::Path;
use std::env;
#[tokio::main]


async fn main() {
    dotenv().ok();    
    let args: Vec<String> = env::args().collect();
    let file_path = Path::new(&args[1]);
    let receive = receive_context(file_path);
    println!("{}", receive.join("\n"));
}


