gbm
=

[![Actions Status](https://github.com/Smithay/gbm.rs/workflows/Continuous%20integration/badge.svg)](https://github.com/Smithay/gbm.rs/actions)
[![Latest version](https://img.shields.io/crates/v/hassle-rs.svg)](https://crates.io/crates/hassle-rs)
[![Documentation](https://docs.rs/hassle-rs/badge.svg)](https://docs.rs/hassle-rs)
[![Lines of code](https://tokei.rs/b1/github/Smithay/gbm.rs)](https://github.com/Smithay/gbm.rs)
![MIT](https://img.shields.io/badge/license-MIT-blue.svg)

### Usage

Add to your `Cargo.toml`:

```toml
gbm = "0.7.0"
```

## Safe `libgbm` bindings for [rust](https://www.rust-lang.org)

The Generic Buffer Manager

This module provides an abstraction that the caller can use to request a
buffer from the underlying memory management system for the platform.

This allows the creation of portable code whilst still allowing access to
the underlying memory manager.

This library is best used in combination with [`drm-rs`](https://github.com/Smithay/drm-rs),
provided through the `drm-support` feature.

### Example

```rust
use drm::control::{self, crtc, framebuffer};
use gbm::{BufferObjectFlags, Device, Format};

#
#
#
#
// ... init your drm device ...
let drm = init_drm_device();

// init a GBM device
let gbm = Device::new(drm).unwrap();

// create a 4x4 buffer
let mut bo = gbm
    .create_buffer_object::<()>(
        1280,
        720,
        Format::Argb8888,
        BufferObjectFlags::SCANOUT | BufferObjectFlags::WRITE,
    )
    .unwrap();

// write something to it (usually use import or egl rendering instead)
let buffer = {
    let mut buffer = Vec::new();
    for i in 0..1280 {
        for _ in 0..720 {
            buffer.push(if i % 2 == 0 { 0 } else { 255 });
        }
    }
    buffer
};
bo.write(&buffer).unwrap();

// create a framebuffer from our buffer
let fb = gbm.add_framebuffer(&bo, 32, 32).unwrap();

#
// display it (and get a crtc, mode and connector before)
gbm.set_crtc(crtc_handle, Some(fb), (0, 0), &[con], Some(mode))
    .unwrap();
```

License: MIT
