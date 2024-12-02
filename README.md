## Safe `libgbm` bindings for [rust](https://www.rust-lang.org)

The Generic Buffer Manager

This module provides an abstraction that the caller can use to request a
buffer from the underlying memory management system for the platform.

This allows the creation of portable code whilst still allowing access to
the underlying memory manager.

This library is best used in combination with [`drm-rs`](https://github.com/Smithay/drm-rs),
provided through the `drm-support` feature.

## Usage

Add to your Cargo.toml

```toml
gbm = "0.18.0"
```

## Example

```rust
use drm::control::{self, crtc, framebuffer};
use gbm::{BufferObjectFlags, Device, Format};

// ... init your drm device ...
let drm = init_drm_device();

// init a GBM device
let gbm = Device::new(drm).unwrap();

// create a buffer
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

// display it (and get a crtc, mode and connector before)
gbm.set_crtc(crtc_handle, Some(fb), (0, 0), &[con], Some(mode))
    .unwrap();
```
