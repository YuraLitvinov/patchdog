/// Function with parameters
/// Multiline
async fn function_with_params(a: i32, b: &str) {}

/// Generic function single-line
const fn generic_function<T>(value: T) {}

    /// Function with lifetimes
    pub async fn lifetime_function<'a>(input: &'a str) -> &'a str {
        input
    }

impl MyTrait for MyStruct {

    /// Multi-line
    /// More text
    fn required_function(&self) {}
    
    /// Single line comment
    fn new() {}

}
