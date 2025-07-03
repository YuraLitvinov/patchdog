#[cfg(test)]
mod tests {
    const TEST_PATH: &str = "/home/runner/work/patchdog/patchdog/tests/data.rs";
    use crate::receive_context;
    use std::path::Path;
    #[test]
    fn testing_seeker_for_use() {
        let string_of_func = "use std::collections::{HashMap,
    HashSet,
    VecDeque};";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(2, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }

    #[test]
    fn testing_seeker_for_zero() {
        let string_of_func: &'static str = "LineOutOfBounds";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(0, path);
        let formatted_receive = receive.unwrap_err().to_string();
        assert!(formatted_receive.contains(string_of_func));
    }
    #[test]
    fn testing_seeker_for_out_of_bounds() {
        let string_of_func: &'static str = "LineOutOfBounds";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(999999, path);
        let formatted_receive = receive.unwrap_err().to_string();
        assert!(formatted_receive.contains(string_of_func));
    }
    #[test]
    fn find_impl() {
        let string_of_func = "impl Item {
    fn new(
        name: &str,
        item_type: ItemType,
        price: f32,
    ) -> Self {
        Self {
            name: name.to_string(),
            item_type,
            price,
            status: Status::Active,
        }
    }

    fn deactivate(
        &mut self
    ) {
        self.status = Status::Inactive;
    }
}";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(43, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }

    #[test]
    fn find_function() {
        let string_of_func = "fn bookshop(
    name: &str,
    item_type: ItemType,
    price: f32,
) -> Self {
    Self {
        name: name.to_string(),
        item_type,
        price,
        status: Status::Active,
    }
}";
        let path = Path::new(TEST_PATH);
        let receive = receive_context(166, path);
        let formatted_receive = receive.unwrap();
        assert_eq!(formatted_receive, string_of_func);
    }
}
