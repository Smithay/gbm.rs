#[cfg(feature = "gen")]
extern crate bindgen;

use std::{env, path::Path};

#[cfg(not(feature = "gen"))]
fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    let bindings_file = Path::new("src")
        .join("platforms")
        .join(&target_os)
        .join(&target_arch)
        .join("gen.rs");

    if bindings_file.is_file() {
        println!(
            "cargo:rustc-env=GBM_SYS_BINDINGS_PATH={}/{}",
            target_os, target_arch
        );
    } else {
        panic!(
            "No prebuilt bindings for target OS `{}` and/or architecture `{}`. Try `gen` feature.",
            target_os, target_arch
        );
    }
}

#[cfg(feature = "gen")]
fn main() {
    const TMP_BIND_PREFIX: &str = "__BINDGEN_TMP_";
    const TMP_BIND_PREFIX_REG: &str = "_BINDGEN_TMP_.*";

    const INCLUDES: &'static [&str] = &["gbm.h"];

    const MACROS: &'static [&str] = &[
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
        .header_contents("bindings.h", &create_header())
        .blacklist_type(TMP_BIND_PREFIX_REG)
        .ctypes_prefix("libc")
        .whitelist_type(r"^gbm_.*$")
        .whitelist_function(r"^gbm_.*$")
        .whitelist_var("GBM_.*|gbm_.*")
        .constified_enum_module(r"^gbm_.*$")
        .generate()
        .unwrap();

    println!("cargo:rerun-if-changed=include/gbm.h");

    // Generate the bindings
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("gen.rs");

    generated.write_to_file(dest_path).unwrap();
}
