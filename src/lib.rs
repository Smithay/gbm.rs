//! # Safe `libgbm` bindings for [rust](https://www.rust-lang.org)
//!
//! The Generic Buffer Manager
//!
//! This module provides an abstraction that the caller can use to request a
//! buffer from the underlying memory management system for the platform.
//!
//! This allows the creation of portable code whilst still allowing access to
//! the underlying memory manager.
//!
//! This library is best used in combination with [`drm-rs`](https://github.com/Smithay/drm-rs),
//! provided through the `drm-support` feature.
//!
//! ## Example
//!
//! ```rust,no_run
//! # extern crate drm;
//! # extern crate gbm;
//! # use drm::control::connector::Info as ConnectorInfo;
//! # use drm::control::Mode;
//! use drm::control::{self, crtc, framebuffer};
//! use gbm::{BufferObjectFlags, Device, Format};
//!
//! # use std::fs::{File, OpenOptions};
//! # use std::os::unix::io::{AsRawFd, RawFd};
//! #
//! # use drm::control::Device as ControlDevice;
//! # use drm::Device as BasicDevice;
//! # struct Card(File);
//! #
//! # impl AsRawFd for Card {
//! #     fn as_raw_fd(&self) -> RawFd {
//! #         self.0.as_raw_fd()
//! #     }
//! # }
//! #
//! # impl BasicDevice for Card {}
//! # impl ControlDevice for Card {}
//! #
//! # fn init_drm_device() -> Card {
//! #     let mut options = OpenOptions::new();
//! #     options.read(true);
//! #     options.write(true);
//! #     let file = options.open("/dev/dri/card0").unwrap();
//! #     Card(file)
//! # }
//! # fn main() {
//! // ... init your drm device ...
//! let drm = init_drm_device();
//!
//! // init a GBM device
//! let gbm = Device::new(drm).unwrap();
//!
//! // create a 4x4 buffer
//! let mut bo = gbm
//!     .create_buffer_object::<()>(
//!         1280,
//!         720,
//!         Format::Argb8888,
//!         BufferObjectFlags::SCANOUT | BufferObjectFlags::WRITE,
//!     )
//!     .unwrap();
//!
//! // write something to it (usually use import or egl rendering instead)
//! let buffer = {
//!     let mut buffer = Vec::new();
//!     for i in 0..1280 {
//!         for _ in 0..720 {
//!             buffer.push(if i % 2 == 0 { 0 } else { 255 });
//!         }
//!     }
//!     buffer
//! };
//! bo.write(&buffer).unwrap();
//!
//! // create a framebuffer from our buffer
//! let fb = gbm.add_framebuffer(&bo, 32, 32).unwrap();
//!
//! # let res_handles = gbm.resource_handles().unwrap();
//! # let con = *res_handles.connectors().iter().next().unwrap();
//! # let crtc_handle = *res_handles.crtcs().iter().next().unwrap();
//! # let connector_info: ConnectorInfo = gbm.get_connector(con).unwrap();
//! # let mode: Mode = connector_info.modes()[0];
//! #
//! // display it (and get a crtc, mode and connector before)
//! gbm.set_crtc(crtc_handle, Some(fb), (0, 0), &[con], Some(mode))
//!     .unwrap();
//! # }
//! ```
#![warn(missing_debug_implementations)]
#![deny(missing_docs)]

extern crate gbm_sys as ffi;
extern crate libc;

#[cfg(feature = "import-wayland")]
extern crate wayland_server;

#[cfg(feature = "drm-support")]
extern crate drm;

extern crate drm_fourcc;

#[macro_use]
extern crate bitflags;

mod buffer_object;
mod device;
mod surface;

pub use self::buffer_object::*;
pub use self::device::*;
pub use self::surface::*;
pub use drm_fourcc::{DrmFourcc as Format, DrmModifier as Modifier};

use std::fmt;
use std::sync::{Arc, Weak};

/// Trait for types that allow to obtain the underlying raw libinput pointer.
pub trait AsRaw<T> {
    /// Receive a raw pointer representing this type.
    fn as_raw(&self) -> *const T;

    #[doc(hidden)]
    fn as_raw_mut(&self) -> *mut T {
        self.as_raw() as *mut _
    }
}

struct PtrDrop<T>(*mut T, Option<Box<dyn FnOnce(*mut T) + Send + 'static>>);

impl<T> Drop for PtrDrop<T> {
    fn drop(&mut self) {
        (self.1.take().unwrap())(self.0);
    }
}

#[derive(Clone)]
pub(crate) struct Ptr<T>(Arc<PtrDrop<T>>);

impl<T> Ptr<T> {
    fn new<F: FnOnce(*mut T) + Send + 'static>(ptr: *mut T, destructor: F) -> Ptr<T> {
        Ptr(Arc::new(PtrDrop(ptr, Some(Box::new(destructor)))))
    }

    fn downgrade(&self) -> WeakPtr<T> {
        WeakPtr(Arc::downgrade(&self.0))
    }
}

impl<T> std::ops::Deref for Ptr<T> {
    type Target = *mut T;

    fn deref(&self) -> &Self::Target {
        &(self.0).0
    }
}

impl<T> fmt::Pointer for Ptr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        fmt::Pointer::fmt(&self.0 .0, f)
    }
}

#[derive(Clone)]
pub(crate) struct WeakPtr<T>(Weak<PtrDrop<T>>);

impl<T> WeakPtr<T> {
    fn upgrade(&self) -> Option<Ptr<T>> {
        self.0.upgrade().map(Ptr)
    }
}

impl<T> fmt::Pointer for WeakPtr<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self.upgrade() {
            Some(x) => fmt::Pointer::fmt(&x, f),
            None => fmt::Pointer::fmt(&std::ptr::null::<T>(), f),
        }
    }
}

unsafe impl<T> Send for WeakPtr<T> where Ptr<T>: Send {}

#[cfg(test)]
mod test {
    fn is_send<T: Send>() {}

    #[test]
    fn device_is_send() {
        is_send::<super::Device<std::fs::File>>();
        is_send::<super::Device<super::FdWrapper>>();
    }

    #[test]
    fn surface_is_send() {
        is_send::<super::Surface<std::fs::File>>();
        is_send::<super::Surface<super::FdWrapper>>();
    }

    #[test]
    fn unmapped_bo_is_send() {
        is_send::<super::BufferObject<()>>();
    }
}
