use std::fs;
pub mod tests;
pub mod binding;

use binding::store_objects;
#[tokio::main]
async fn main() {
    let patch_text = fs::read("/home/yurii-sama/Desktop/patchdog/patch.patch").expect("Failed to read patch file");
    let surplus = store_objects("", &patch_text).unwrap();
  /*   for each in surplus {
        println!("{}", each.name);
        let obj = each.object_type;
        for each in obj {
            println!("{:?}", each);
        }
    }
    */
    println!("{:?}", surplus[0]);
}
