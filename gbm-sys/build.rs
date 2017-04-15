extern crate bindgen;

fn main()
{
    // Setup bindings builder
    let generated = bindgen::builder()
        .header("include/gbm.h")
        .no_unstable_rust()
        .ctypes_prefix("libc")
        .whitelisted_type(r"^gbm_.*$")
        .whitelisted_function(r"^gbm_.*$")
        .generate().unwrap();

    println!("cargo:rustc-link-lib=dylib=gbm");

    // Generate the bindings
    generated.write_to_file("src/gen.rs").unwrap();
}
