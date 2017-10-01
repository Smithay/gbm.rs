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

`gbm = "0.2.2"`

## Example

```rust,no_run
extern crate drm;
extern crate gbm;

use drm::control::{crtc, framebuffer};
use gbm::{Device, Format, BufferObjectFlags};

// ... init your drm device ...
let drm = init_drm_device();

// init a gbm device
let gbm = Device::new_from_drm(&drm).unwrap();

// create a buffer
let mut bo = gbm.create_buffer_object::<()>(
            1280, 720,
            Format::ARGB8888,
            &[
                BufferObjectFlags::Scanout,
                BufferObjectFlags::Write,
            ]).unwrap();

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
let fb_info = framebuffer::create(&drm, &bo).unwrap();

// display it (and get a crtc, mode and connector before)
crtc::set(&drm, crtc_handle, fb_info.handle(), &[con], (0, 0), Some(mode)).unwrap();
```
