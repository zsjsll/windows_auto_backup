use std::{env, path::Path};

fn main() {
    let profile = env::var("PROFILE").unwrap(); // debug 或 release
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let target_dir = Path::new(&manifest_dir).join("target").join(&profile);

    println!("cargo:warning=🔥 11111111111111111111111111111111");
    println!("cargo:warning={}",&manifest_dir);
    // panic!("BUILD_RS_IS_RUNNING_PANIC");
}
