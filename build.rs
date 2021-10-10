use std::{env, path::PathBuf};

fn main() {
    // cc::Build::new().file("src/std.c").compile("std");
    let bindings = bindgen::Builder::default()
        .header("src/std.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .unwrap();
    println!("cargo:rerun-if-changed=src/std.h");
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}
