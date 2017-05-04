extern crate gbm_sys as ffi;
extern crate libc;

#[cfg(feature = "import_wayland")]
extern crate wayland_server;

#[cfg(feature = "import_egl")]
extern crate egli;

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
    /// If unsure using `()` is always a safe option..
    ///
    /// ## Unsafety
    ///
    /// If the pointer is pointing to a different struct, invalid memory or `NULL` the returned
    /// struct may panic on use or cause other undefined behavior.
    ///
    unsafe fn from_raw(*mut T) -> Self;
}

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
}
