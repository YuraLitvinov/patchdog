pub mod binding;
#[cfg(test)]
pub mod tests;
use crate::binding::get_patch_data;
#[tokio::main]
async fn main() {
    let a = get_patch_data(
        "/home/yurii-sama/patchdog/patch.patch",
        "/home/yurii-sama/patchdog/",
    )
    .expect("msg");
    for each in a {
        println!("{:?}", each);
    }
}
