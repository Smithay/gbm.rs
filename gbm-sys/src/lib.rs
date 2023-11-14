#![allow(non_camel_case_types, non_upper_case_globals)]
// Allowed this because some bindgen tests looks like
// it tries to dereference null pointers but actually
// it is not so.
#![cfg_attr(test, allow(deref_nullptr))]

extern crate libc;

#[cfg(feature = "gen")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(not(feature = "gen"))]
include!("bindings.rs");

#[link(name = "gbm")]
extern "C" {}
