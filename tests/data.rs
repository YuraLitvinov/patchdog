// Regular private function
fn regular_function() {}

// Public function
pub fn public_function() {}

// Function with return type
fn function_with_return() -> i32 {
    0
}
fn required_function() {
}

// Function with parameters
fn function_with_params(a: i32, b: &str) {}

// Generic function
fn generic_function<T>(value: T) {}

// Function with lifetimes
fn lifetime_function<'a>(input: &'a str) -> &'a str {
    input
}

// Const function
const fn const_function(x: i32) -> i32 {
    x
}

// Async function
async fn async_function() {}

// Unsafe function
unsafe fn unsafe_function() {}

// Extern function declaration (FFI)
extern "C" fn extern_c_function() {}

// Function returning a Result
fn result_function() -> Result<(), String> {
    Ok(())
}

// Method in impl
struct MyStruct;

impl MyStruct {
    fn method(&self) {}

    pub fn public_method(&self) {}

    fn static_method() {}

    fn method_with_lifetime<'a>(&'a self, input: &'a str) -> &'a str {
        input
    }

    fn method_with_generic<T>(&self, value: T) {}
}

// Trait with functions
trait MyTrait {
    fn required_function(&self);

    fn default_function(&self) {
        // Default implementation
    }
}

impl MyTrait for MyStruct {
    fn required_function(&self) {}
}