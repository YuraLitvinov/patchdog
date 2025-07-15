use crate::binding::{make_export};
pub mod binding;
#[cfg(test)]
pub mod tests;
#[tokio::main]
async fn main() {
    //let _call = patch_data_argument();
    let make = make_export(&["/home/yurii-sama/patchdog/crates/patchdog/src/main.rs"])
        .expect("err");
    println!("{:?}", make);

}
