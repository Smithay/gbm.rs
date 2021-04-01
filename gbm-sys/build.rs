#[cfg(feature = "gen")]
extern crate bindgen;

#[cfg(feature = "gen")]
use std::env;
#[cfg(feature = "gen")]
use std::path::Path;

#[cfg(not(feature = "gen"))]
fn main() {}

#[cfg(feature = "gen")]
fn main()
{
    // Setup bindings builder
    let generated = bindgen::builder()
        .header("include/gbm.h")
        .ctypes_prefix("libc")
        .whitelist_type(r"^gbm_.*$")
        .whitelist_function(r"^gbm_.*$")
        .constified_enum("gbm_bo_flags")
        .constified_enum_module("gbm_bo_transfer_flags")
        .generate()
        .unwrap();

    println!("cargo:rerun-if-changed=include/gbm.h");

    // Generate the bindings
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("gen.rs");

    generated.write_to_file(dest_path).unwrap();
}
