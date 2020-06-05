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
//! extern crate drm;
//! extern crate gbm;
//!
//! use drm::control::{crtc, framebuffer};
//! # use drm::control::{Mode, ResourceInfo};
//! # use drm::control::connector::Info as ConnectorInfo;
//! use gbm::{Device, Format, BufferObjectFlags};
//!
//! # use std::fs::{OpenOptions, File};
//! # use std::os::unix::io::{AsRawFd, RawFd};
//! #
//! # use drm::Device as BasicDevice;
//! # use drm::control::Device as ControlDevice;
//! # struct Card(File);
//! #
//! # impl AsRawFd for Card {
//! #     fn as_raw_fd(&self) -> RawFd { self.0.as_raw_fd() }
//! # }
//! #
//! # impl BasicDevice for Card { }
//! # impl ControlDevice for Card { }
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
//! // init a gbm device
//! let gbm = Device::new(drm).unwrap();
//!
//! // create a 4x4 buffer
//! let mut bo = gbm.create_buffer_object::<()>(
//!             1280, 720,
//!             Format::ARGB8888,
//!             BufferObjectFlags::SCANOUT | BufferObjectFlags::WRITE,
//!             ).unwrap();
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
//! let fb_info = framebuffer::create(&gbm, &bo).unwrap();
//!
//! # let res_handles = gbm.resource_handles().unwrap();
//! # let con = *res_handles.connectors().iter().next().unwrap();
//! # let crtc_handle = *res_handles.crtcs().iter().next().unwrap();
//! # let connector_info: ConnectorInfo = gbm.resource_info(con).unwrap();
//! # let mode: Mode = connector_info.modes()[0];
//! #
//! // display it (and get a crtc, mode and connector before)
//! crtc::set(&gbm, crtc_handle, fb_info.handle(), &[con], (0, 0), Some(mode)).unwrap();
//! # }
//! ```

#![deny(missing_docs)]

extern crate gbm_sys as ffi;
extern crate libc;

#[cfg(feature = "import-wayland")]
extern crate wayland_server;

#[cfg(feature = "drm-support")]
extern crate drm;

#[macro_use]
extern crate bitflags;

mod device;
mod buffer_object;
mod surface;

pub use self::buffer_object::*;
pub use self::device::*;
pub use self::surface::*;

use std::sync::{Arc, Weak};

/// Trait for types that allow to optain the underlying raw libinput pointer.
pub trait AsRaw<T> {
    /// Receive a raw pointer representing this type.
    fn as_raw(&self) -> *const T;

    #[doc(hidden)]
    fn as_raw_mut(&self) -> *mut T {
        self.as_raw() as *mut _
    }
}

/// Possible pixel formats used
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    C8,
    R8,
    GR88,

    RGB332,
    BGR233,

    XRGB4444,
    XBGR4444,
    RGBX4444,
    BGRX4444,

    ARGB4444,
    ABGR4444,
    RGBA4444,
    BGRA4444,

    XRGB1555,
    XBGR1555,
    RGBX5551,
    BGRX5551,

    ARGB1555,
    ABGR1555,
    RGBA5551,
    BGRA5551,

    RGB565,
    BGR565,

    XRGB8888,
    XBGR8888,
    RGBX8888,
    BGRX8888,

    ARGB8888,
    ABGR8888,
    RGBA8888,
    BGRA8888,

    XRGB2101010,
    XBGR2101010,
    RGBX1010102,
    BGRX1010102,

    ARGB2101010,
    ABGR2101010,
    RGBA1010102,
    BGRA1010102,

    YUYV,
    YVYU,
    UYVY,
    VYUY,

    AYUV,
}

impl Format {
    #[doc(hidden)]
    pub fn as_ffi(&self) -> u32 {
        use Format::*;
        match *self {
            C8 => ::ffi::GBM_FORMAT_C8,
            R8 => ::ffi::GBM_FORMAT_R8,
            GR88 => ::ffi::GBM_FORMAT_GR88,

            RGB332 => ::ffi::GBM_FORMAT_RGB332,
            BGR233 => ::ffi::GBM_FORMAT_BGR233,

            XRGB4444 => ::ffi::GBM_FORMAT_XRGB4444,
            XBGR4444 => ::ffi::GBM_FORMAT_XBGR4444,
            RGBX4444 => ::ffi::GBM_FORMAT_RGBX4444,
            BGRX4444 => ::ffi::GBM_FORMAT_BGRX4444,

            ARGB4444 => ::ffi::GBM_FORMAT_ARGB4444,
            ABGR4444 => ::ffi::GBM_FORMAT_ABGR4444,
            RGBA4444 => ::ffi::GBM_FORMAT_RGBA4444,
            BGRA4444 => ::ffi::GBM_FORMAT_BGRA4444,

            XRGB1555 => ::ffi::GBM_FORMAT_XRGB1555,
            XBGR1555 => ::ffi::GBM_FORMAT_XBGR1555,
            RGBX5551 => ::ffi::GBM_FORMAT_RGBX5551,
            BGRX5551 => ::ffi::GBM_FORMAT_BGRX5551,

            ARGB1555 => ::ffi::GBM_FORMAT_ARGB1555,
            ABGR1555 => ::ffi::GBM_FORMAT_ABGR1555,
            RGBA5551 => ::ffi::GBM_FORMAT_RGBA4444,
            BGRA5551 => ::ffi::GBM_FORMAT_RGBA5551,

            RGB565 => ::ffi::GBM_FORMAT_RGB565,
            BGR565 => ::ffi::GBM_FORMAT_BGR565,

            XRGB8888 => ::ffi::GBM_FORMAT_XRGB8888,
            XBGR8888 => ::ffi::GBM_FORMAT_XBGR8888,
            RGBX8888 => ::ffi::GBM_FORMAT_RGBX8888,
            BGRX8888 => ::ffi::GBM_FORMAT_BGRX8888,

            ARGB8888 => ::ffi::GBM_FORMAT_ARGB8888,
            ABGR8888 => ::ffi::GBM_FORMAT_ABGR8888,
            RGBA8888 => ::ffi::GBM_FORMAT_RGBA8888,
            BGRA8888 => ::ffi::GBM_FORMAT_BGRA8888,

            XRGB2101010 => ::ffi::GBM_FORMAT_XRGB2101010,
            XBGR2101010 => ::ffi::GBM_FORMAT_XBGR2101010,
            RGBX1010102 => ::ffi::GBM_FORMAT_RGBX1010102,
            BGRX1010102 => ::ffi::GBM_FORMAT_BGRX1010102,

            ARGB2101010 => ::ffi::GBM_FORMAT_ARGB2101010,
            ABGR2101010 => ::ffi::GBM_FORMAT_ABGR2101010,
            RGBA1010102 => ::ffi::GBM_FORMAT_RGBA1010102,
            BGRA1010102 => ::ffi::GBM_FORMAT_BGRA1010102,

            YUYV => ::ffi::GBM_FORMAT_YUYV,
            YVYU => ::ffi::GBM_FORMAT_YVYU,
            UYVY => ::ffi::GBM_FORMAT_UYVY,
            VYUY => ::ffi::GBM_FORMAT_VYUY,

            AYUV => ::ffi::GBM_FORMAT_AYUV,
        }
    }

    #[doc(hidden)]
    pub fn from_ffi(raw: u32) -> Option<Format> {
        use Format::*;

        match raw {
            x if x == ::ffi::GBM_FORMAT_C8 as u32 => Some(C8),
            x if x == ::ffi::GBM_FORMAT_R8 as u32 => Some(R8),
            x if x == ::ffi::GBM_FORMAT_GR88 as u32 => Some(GR88),

            x if x == ::ffi::GBM_FORMAT_RGB332 as u32 => Some(RGB332),
            x if x == ::ffi::GBM_FORMAT_BGR233 as u32 => Some(BGR233),

            x if x == ::ffi::GBM_FORMAT_XRGB4444 as u32 => Some(XRGB4444),
            x if x == ::ffi::GBM_FORMAT_XBGR4444 as u32 => Some(XBGR4444),
            x if x == ::ffi::GBM_FORMAT_RGBX4444 as u32 => Some(RGBX4444),
            x if x == ::ffi::GBM_FORMAT_BGRX4444 as u32 => Some(BGRX4444),

            x if x == ::ffi::GBM_FORMAT_ARGB4444 as u32 => Some(ARGB4444),
            x if x == ::ffi::GBM_FORMAT_ABGR4444 as u32 => Some(ABGR4444),
            x if x == ::ffi::GBM_FORMAT_RGBA4444 as u32 => Some(RGBA4444),
            x if x == ::ffi::GBM_FORMAT_BGRA4444 as u32 => Some(BGRA4444),

            x if x == ::ffi::GBM_FORMAT_XRGB1555 as u32 => Some(XRGB1555),
            x if x == ::ffi::GBM_FORMAT_XBGR1555 as u32 => Some(XBGR1555),
            x if x == ::ffi::GBM_FORMAT_RGBX5551 as u32 => Some(RGBX5551),
            x if x == ::ffi::GBM_FORMAT_BGRX5551 as u32 => Some(BGRX5551),

            x if x == ::ffi::GBM_FORMAT_ARGB1555 as u32 => Some(ARGB1555),
            x if x == ::ffi::GBM_FORMAT_ABGR1555 as u32 => Some(ABGR1555),
            x if x == ::ffi::GBM_FORMAT_RGBA5551 as u32 => Some(RGBA5551),
            x if x == ::ffi::GBM_FORMAT_BGRA5551 as u32 => Some(BGRA5551),

            x if x == ::ffi::GBM_FORMAT_RGB565 as u32 => Some(RGB565),
            x if x == ::ffi::GBM_FORMAT_BGR565 as u32 => Some(BGR565),

            x if x == ::ffi::GBM_FORMAT_XRGB8888 as u32 => Some(XRGB8888),
            x if x == ::ffi::GBM_FORMAT_XBGR8888 as u32 => Some(XBGR8888),
            x if x == ::ffi::GBM_FORMAT_RGBX8888 as u32 => Some(RGBX8888),
            x if x == ::ffi::GBM_FORMAT_BGRX8888 as u32 => Some(BGRX8888),

            x if x == ::ffi::GBM_FORMAT_ARGB8888 as u32 => Some(ARGB8888),
            x if x == ::ffi::GBM_FORMAT_ABGR8888 as u32 => Some(ABGR8888),
            x if x == ::ffi::GBM_FORMAT_RGBA8888 as u32 => Some(RGBA8888),
            x if x == ::ffi::GBM_FORMAT_BGRA8888 as u32 => Some(BGRA8888),

            x if x == ::ffi::GBM_FORMAT_XRGB2101010 as u32 => Some(XRGB2101010),
            x if x == ::ffi::GBM_FORMAT_XBGR2101010 as u32 => Some(XBGR2101010),
            x if x == ::ffi::GBM_FORMAT_RGBX1010102 as u32 => Some(RGBX1010102),
            x if x == ::ffi::GBM_FORMAT_BGRX1010102 as u32 => Some(BGRX1010102),

            x if x == ::ffi::GBM_FORMAT_ARGB2101010 as u32 => Some(ARGB2101010),
            x if x == ::ffi::GBM_FORMAT_ABGR2101010 as u32 => Some(ABGR2101010),
            x if x == ::ffi::GBM_FORMAT_RGBA1010102 as u32 => Some(RGBA1010102),
            x if x == ::ffi::GBM_FORMAT_BGRA1010102 as u32 => Some(BGRA1010102),

            x if x == ::ffi::GBM_FORMAT_YUYV as u32 => Some(YUYV),
            x if x == ::ffi::GBM_FORMAT_YVYU as u32 => Some(YVYU),
            x if x == ::ffi::GBM_FORMAT_UYVY as u32 => Some(UYVY),
            x if x == ::ffi::GBM_FORMAT_VYUY as u32 => Some(VYUY),

            x if x == ::ffi::GBM_FORMAT_AYUV as u32 => Some(AYUV),

            _ => None,
        }
    }
}

struct PtrDrop<T>(*mut T, Option<Box<dyn FnOnce(*mut T) + 'static>>);

impl<T> Drop for PtrDrop<T> {
    fn drop(&mut self) {
        (self.1.take().unwrap())(self.0);
    }
}

pub(crate) struct Ptr<T>(Arc<PtrDrop<T>>);

impl<T> Ptr<T> {
    fn new<F: FnOnce(*mut T) + 'static>(ptr: *mut T, destructor: F) -> Ptr<T> {
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

#[derive(Clone)]
pub(crate) struct WeakPtr<T>(Weak<PtrDrop<T>>);

impl<T> WeakPtr<T> {
    fn upgrade(&self) -> Option<Ptr<T>> {
        self.0.upgrade().map(Ptr)
    }
}

unsafe impl<T> Send for WeakPtr<T> where Ptr<T>: Send {}

#[cfg(test)]
mod test {
    fn is_send<T: Send>() {}

    #[test]
    fn device_is_send() {
        is_send::<super::Device<std::fs::File>>();
    }

    #[test]
    fn surface_is_send() {
        is_send::<super::Surface<std::fs::File>>();
    }
}