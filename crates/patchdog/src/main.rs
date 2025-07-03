use dotenv::dotenv;
use patchdog::receive_context;
use std::env;
use std::path::Path;
pub mod tests;
#[tokio::main]

async fn main() {
    dotenv().ok();
    let args: Vec<String> = env::args().collect();
    let file_path = Path::new(&args[2]);
    let line_from = match args[1].parse::<usize>() {
        Ok(line_from) => line_from,
        Err(why) => {
            eprintln!("{}", why);
            return;
        }
    };
    println!("{}", line_from);
    let receive = receive_context(line_from, file_path);
    let formatted_receive = receive.unwrap();
    println!("{}", formatted_receive);
}

