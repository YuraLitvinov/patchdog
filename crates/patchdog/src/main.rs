use dotenv::dotenv;
use patchdog::receive_context;
use std::env;
use std::path::Path;
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
    let formatted_receive = receive.to_string();
    println!("{}", formatted_receive);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn testing_seeker_for_basic() {
        let string_of_func = "use filesystem_parsing::{extract_function, ObjectRange};";
        let path = Path::new("/home/runner/work/patchdog/patchdog/crates/patchdog/src/lib.rs");
        let receive = receive_context(1, path);
        let formatted_receive = receive.to_string();
        assert_eq!(formatted_receive, string_of_func);
    }
    #[test]
    fn testing_seeker_for_zero() {
        let string_of_func: &'static str = "Index out of bounds";
        let path = Path::new("/home/runner/work/patchdog/patchdog/crates/patchdog/src/lib.rs");
        let receive = receive_context(0, path);
        let formatted_receive = receive.to_string();
        assert_eq!(formatted_receive, string_of_func);
    }
    #[test]
    fn testing_seeker_for_out_of_bounds() {
        let string_of_func: &'static str = "Index out of bounds";
        let path = Path::new("/home/runner/work/patchdog/patchdog/crates/patchdog/src/lib.rs");
        let receive = receive_context(10000, path);
        let formatted_receive = receive.to_string();
        assert_eq!(formatted_receive, string_of_func);
    }
}
