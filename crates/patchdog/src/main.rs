use std::fs;
pub mod tests;
pub mod binding;

use binding::store_objects;
#[tokio::main]
async fn main() {
    let patch_text = fs::read("/home/yurii-sama/Desktop/patchdog/patch.patch").expect("Failed to read patch file");
    let surplus = store_objects("/home/yurii-sama/Desktop/patchdog/", &patch_text).unwrap();
     for each in surplus {
        println!("{:?}", each);
    }
}
