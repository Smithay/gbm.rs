//!
//! # Safe `libgbm` bindings for [rust](https://www.rust-lang.org)
//!
//! The Generic Buffer Manager
//!
//! This module provides an abstraction that the caller can use to request a
//!buffer from the underlying memory management system for the platform.
//!
//! This allows the creation of portable code whilst still allowing access to
//! the underlying memory manager.

#![deny(missing_docs)]

extern crate gbm_sys as ffi;
extern crate libc;

#[cfg(feature = "import_wayland")]
extern crate wayland_server;

#[cfg(feature = "import_egl")]
extern crate egli;

#[cfg(feature = "drm-support")]
extern crate drm;

mod device;
mod buffer_object;
mod surface;

pub use self::device::*;
pub use self::buffer_object::*;
pub use self::surface::*;

/// Trait for types that allow to optain the underlying raw libinput pointer.
pub trait AsRaw<T> {
    /// Receive a raw pointer representing this type.
    fn as_raw(&self) -> *const T;

    #[doc(hidden)]
    fn as_raw_mut(&self) -> *mut T {
        self.as_raw() as *mut _
    }
}

/// Trait for types that allow to be initialized from a raw pointer
pub trait FromRaw<T> {
    /// Create a new instance of this type from a raw pointer.
    ///
    /// ## Warning
    ///
    /// If you make use of [`Userdata`](./trait.Userdata.html) make sure you use the correct types
    /// to allow receiving the set userdata. When dealing with raw pointers initialized by other
    /// libraries this must be done extra carefully to select a correct representation.
    ///
    /// If unsure using `()` is always a safe option.
    ///
    /// ## Unsafety
    ///
    /// If the pointer is pointing to a different struct, invalid memory or `NULL` the returned
    /// struct may panic on use or cause other undefined behavior.
    ///
    /// Effectively cloning objects by using `as_raw` and `from_raw` is also unsafe as
    /// a double free may occur.
    unsafe fn from_raw(*mut T) -> Self;
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

            _ => None
        }
    }
}
