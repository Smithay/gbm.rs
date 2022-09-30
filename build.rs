#[cfg(feature = "auto-detect")]
fn main() {
    let has_gbm_bo_get_fd_for_plane = cc::Build::new()
        .file("test_gbm_bo_get_fd_for_plane.c")
        .warnings_into_errors(true)
        .try_compile("test_gbm_bo_get_fd_for_plane")
        .is_ok();

    if has_gbm_bo_get_fd_for_plane {
        println!("cargo:rustc-cfg=HAS_GBM_BO_GET_FD_FOR_PLANE");
    }
}

#[cfg(not(feature = "auto-detect"))]
fn main() {}
