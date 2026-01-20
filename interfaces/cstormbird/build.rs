extern crate cbindgen;

use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:warning=BUILD SCRIPT RUNNING");
    
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let include_dir = PathBuf::from(&crate_dir).join("include");
    std::fs::create_dir_all(&include_dir).expect("Failed to create include directory");

    cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_config(cbindgen::Config::from_file("cbindgen.toml").unwrap())
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file(include_dir.join("cstormbird.h"));

    println!("cargo:rerun-if-changed=src/");
}
