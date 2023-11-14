#[cfg(feature = "gen")]
extern crate bindgen;

#[cfg(not(feature = "gen"))]
fn main() {}

#[cfg(feature = "gen")]
fn main() {
    use std::{env, path::Path};

    const TMP_BIND_PREFIX: &str = "__BINDGEN_TMP_";
    const TMP_BIND_PREFIX_REG: &str = "_BINDGEN_TMP_.*";

    const INCLUDES: &[&str] = &["gbm.h"];

    const MACROS: &[&str] = &[
        "GBM_BO_IMPORT_WL_BUFFER",
        "GBM_BO_IMPORT_EGL_IMAGE",
        "GBM_BO_IMPORT_FD",
        "GBM_BO_IMPORT_FD_MODIFIER",
    ];

    // Applies a formatting function over a slice of strings,
    // concatenating them on separate lines into a single String
    fn apply_formatting<I, F>(iter: I, f: F) -> String
    where
        I: Iterator,
        I::Item: AsRef<str>,
        F: Fn(&str) -> String,
    {
        iter.fold(String::new(), |acc, x| acc + &f(x.as_ref()) + "\n")
    }

    // Create a name for a temporary value
    fn tmp_val(name: &str) -> String {
        format!("{}{}", TMP_BIND_PREFIX, name)
    }

    // Create a C include directive
    fn include(header: &str) -> String {
        format!("#include <{}>", header)
    }

    // Create a C constant
    fn decl_const(ty: &str, name: &str, value: &str) -> String {
        format!("const {} {} = {};", ty, name, value)
    }

    // Create a C macro definition
    fn define_macro(name: &str, val: &str) -> String {
        format!("#define {} {}", name, val)
    }

    // Create a C undefinition
    fn undefine_macro(name: &str) -> String {
        format!("#undef {}", name)
    }

    // Rebind a C macro as a constant
    // Required for some macros that won't get generated
    fn rebind_macro(name: &str) -> String {
        let tmp_name = tmp_val(name);
        format!(
            "{}\n{}\n{}\n{}",
            decl_const("unsigned int", &tmp_name, name),
            undefine_macro(name),
            decl_const("unsigned int", name, &tmp_name),
            define_macro(name, name)
        )
    }

    // Fully create the header
    fn create_header() -> String {
        apply_formatting(INCLUDES.iter(), include) + &apply_formatting(MACROS.iter(), rebind_macro)
    }

    // Setup bindings builder
    let generated = bindgen::builder()
        .clang_arg("-Iinclude")
        .header_contents("bindings.h", &create_header())
        .blocklist_type(TMP_BIND_PREFIX_REG)
        .ctypes_prefix("libc")
        .allowlist_type("^gbm_.*$")
        .allowlist_function("^gbm_.*$")
        .allowlist_var("GBM_.*|gbm_.*")
        .constified_enum_module("^gbm_.*$")
        // Layout tests are incorrect across architectures
        .layout_tests(false)
        .generate()
        .unwrap();

    println!("cargo:rerun-if-changed=include/gbm.h");

    // Generate the bindings
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("bindings.rs");

    generated.write_to_file(dest_path).unwrap();

    #[cfg(feature = "update_bindings")]
    {
        use std::fs;

        let bind_file = Path::new(&out_dir).join("bindings.rs");
        let dest_file = "src/bindings.rs";

        fs::copy(bind_file, dest_file).unwrap();
    }
}
