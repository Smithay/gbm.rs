#![allow(non_camel_case_types, non_upper_case_globals)]

extern crate libc;

#[cfg(feature = "gen")]
include!(concat!(env!("OUT_DIR"), "/gen.rs"));

#[cfg(all(not(feature = "gen"), target_os = "linux", target_arch = "x86_64"))]
include!(concat!("platforms/linux/x86_64/gen.rs"));

#[link(name = "gbm")]
extern "C" {}
