pub mod binding;
#[cfg(test)]
pub mod tests;
use crate::binding::patch_interface;
#[tokio::main]
async fn main() {
    let a = patch_interface("/home/yurii-sama/patchdog/patch.patch", "/home/yurii-sama/patchdog/"); 
    for each in a {
        println!("{:?}", each);
    }
}

