#[cfg(test)]
mod tests {
    use std::path::Path;
    use crate::receive_context;
    #[test]
    fn testing_seeker_for_basic() {
        let string_of_func = "use filesystem_parsing::parse_all_rust_items;";
        let path = Path::new("/home/runner/work/patchdog/patchdog/crates/patchdog/src/lib.rs");
        let receive = receive_context(1, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }
    #[test]
    fn testing_seeker_for_zero() {
        let string_of_func: &'static str = "LineOutOfBounds { line_index: 0 }";
        let path = Path::new("/home/runner/work/patchdog/patchdog/crates/patchdog/src/lib.rs");
        let receive = receive_context(0, path);
        let formatted_receive = receive.unwrap();
        assert_ne!(formatted_receive, string_of_func);
    }
    #[test]
    fn testing_seeker_for_out_of_bounds() {
        let string_of_func: &'static str = "LineOutOfBounds { line_index: 10000 }";
        let path = Path::new("/home/runner/work/patchdog/patchdog/crates/patchdog/src/lib.rs");
        let receive = receive_context(10000, path);
        let formatted_receive = receive.unwrap();
        assert_ne!(formatted_receive, string_of_func);
    }
}
