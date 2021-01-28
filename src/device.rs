use {AsRaw, BufferObject, BufferObjectFlags, Format, Surface};

use libc::c_void;

use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io::{Error as IoError, Result as IoResult};
use std::ops::{Deref, DerefMut};
use std::os::unix::io::{AsRawFd, RawFd};
use std::rc::Rc;

#[cfg(feature = "glutin-support")]
use glutin_interface::{
    GbmWindowParts, NativeDisplay, NativeWindowSource, RawDisplay, Seal, WaylandWindowParts, X11WindowParts,
};
#[cfg(feature = "glutin-support")]
use std::marker::PhantomData;
#[cfg(feature = "glutin-support")]
use std::sync::Arc;
#[cfg(feature = "glutin-support")]
use winit_types::platform::OsError;
#[cfg(feature = "glutin-support")]
use winit_types::{
    dpi::PhysicalSize,
    error::{Error, ErrorType},
};

#[cfg(feature = "import-wayland")]
use wayland_client::{protocol::wl_buffer::WlBuffer, Proxy};

#[cfg(feature = "import-egl")]
/// An EGLImage handle
pub type EGLImage = *mut c_void;

#[cfg(feature = "drm-support")]
use drm::control::Device as DrmControlDevice;
#[cfg(feature = "drm-support")]
use drm::Device as DrmDevice;

/// Type wrapping a foreign file destructor
pub struct FdWrapper(RawFd);

impl AsRawFd for FdWrapper {
    fn as_raw_fd(&self) -> RawFd {
        self.0
    }
}

/// An open GBM device
pub struct Device<T: AsRawFd + 'static> {
    fd: T,
    ffi: Rc<*mut ::ffi::gbm_device>,
}

impl<T: AsRawFd + 'static> AsRawFd for Device<T> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { ::ffi::gbm_device_get_fd(*self.ffi) }
    }
}

impl<T: AsRawFd + 'static> AsRaw<::ffi::gbm_device> for Device<T> {
    fn as_raw(&self) -> *const ::ffi::gbm_device {
        *self.ffi
    }
}

impl<T: AsRawFd + 'static> Deref for Device<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.fd
    }
}

impl<T: AsRawFd + 'static> DerefMut for Device<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.fd
    }
}

impl Device<FdWrapper> {
    /// Open a GBM device from a given unix file descriptor.
    ///
    /// The file descriptor passed in is used by the backend to communicate with
    /// platform for allocating the memory. For allocations using DRI this would be
    /// the file descriptor returned when opening a device such as /dev/dri/card0.
    ///
    /// # Unsafety
    ///
    /// The lifetime of the resulting device depends on the ownership of the file descriptor.
    /// Closing the file descriptor before dropping the Device will lead to undefined behavior.
    pub unsafe fn new_from_fd(fd: RawFd) -> IoResult<Device<FdWrapper>> {
        let ptr = ::ffi::gbm_create_device(fd);
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Device {
                fd: FdWrapper(fd),
                ffi: Rc::new(ptr),
            })
        }
    }
}

impl<T: AsRawFd + 'static> Device<T> {
    /// Open a GBM device from a given open DRM device.
    ///
    /// The underlying file descriptor passed in is used by the backend to communicate with
    /// platform for allocating the memory. For allocations using DRI this would be
    /// the file descriptor returned when opening a device such as /dev/dri/card0.
    pub fn new(fd: T) -> IoResult<Device<T>> {
        let ptr = unsafe { ::ffi::gbm_create_device(fd.as_raw_fd()) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Device {
                fd,
                ffi: Rc::new(ptr),
            })
        }
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &str {
        unsafe {
            CStr::from_ptr(::ffi::gbm_device_get_backend_name(*self.ffi))
                .to_str()
                .expect("GBM passed invalid utf8 string")
        }
    }

    /// Test if a format is supported for a given set of usage flags
    pub fn is_format_supported(&self, format: Format, usage: BufferObjectFlags) -> bool {
        unsafe { ::ffi::gbm_device_is_format_supported(*self.ffi, format.as_ffi(), usage.bits()) != 0 }
    }

    /// Allocate a new surface object
    pub fn create_surface<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        usage: BufferObjectFlags,
    ) -> IoResult<Surface<U>> {
        let ptr =
            unsafe { ::ffi::gbm_surface_create(*self.ffi, width, height, format.as_ffi(), usage.bits()) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::new(ptr, Rc::downgrade(&self.ffi)) })
        }
    }

    ///  Allocate a buffer object for the given dimensions
    pub fn create_buffer_object<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let ptr = unsafe { ::ffi::gbm_bo_create(*self.ffi, width, height, format.as_ffi(), usage.bits()) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, Rc::downgrade(&self.ffi)) })
        }
    }

    /// Create a gbm buffer object from a wayland buffer
    ///
    /// This function imports a foreign `WlBuffer` object and creates a new gbm
    /// buffer object for it.
    /// This enabled using the foreign object with a display API such as KMS.
    ///
    /// The gbm bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    #[cfg(feature = "import-wayland")]
    pub fn import_buffer_object_from_wayland<U: 'static>(
        &self,
        buffer: &Proxy<WlBuffer>,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let ptr = unsafe {
            ::ffi::gbm_bo_import(
                *self.ffi,
                ::ffi::GBM_BO_IMPORT::WL_BUFFER as u32,
                buffer.c_ptr() as *mut _,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, Rc::downgrade(&self.ffi)) })
        }
    }

    /// Create a gbm buffer object from an egl buffer
    ///
    /// This function imports a foreign `EGLImage` object and creates a new gbm
    /// buffer object for it.
    /// This enabled using the foreign object with a display API such as KMS.
    ///
    /// The gbm bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    ///
    /// ## Unsafety
    ///
    /// The given EGLImage is a raw pointer. Passing null or an invalid EGLImage will
    /// cause undefined behavior.
    #[cfg(feature = "import-egl")]
    pub unsafe fn import_buffer_object_from_egl<U: 'static>(
        &self,
        buffer: EGLImage,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let ptr = ::ffi::gbm_bo_import(
            *self.ffi,
            ::ffi::GBM_BO_IMPORT::EGL_IMAGE as u32,
            buffer,
            usage.bits(),
        );
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(BufferObject::new(ptr, Rc::downgrade(&self.ffi)))
        }
    }

    /// Create a gbm buffer object from an dma buffer
    ///
    /// This function imports a foreign dma buffer from an open file descriptor
    /// and creates a new gbm buffer object for it.
    /// This enabled using the foreign object with a display API such as KMS.
    ///
    /// The gbm bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    pub fn import_buffer_object_from_dma_buf<U: 'static>(
        &self,
        buffer: RawFd,
        width: u32,
        height: u32,
        stride: u32,
        format: Format,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let mut fd_data = ::ffi::gbm_import_fd_data {
            fd: buffer,
            width,
            height,
            stride,
            format: format.as_ffi(),
        };

        let ptr = unsafe {
            ::ffi::gbm_bo_import(
                *self.ffi,
                ::ffi::GBM_BO_IMPORT::FD as u32,
                &mut fd_data as *mut ::ffi::gbm_import_fd_data as *mut _,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, Rc::downgrade(&self.ffi)) })
        }
    }
}

#[cfg(feature = "drm-support")]
impl<T: DrmDevice + AsRawFd + 'static> DrmDevice for Device<T> {}

#[cfg(feature = "drm-support")]
impl<T: DrmControlDevice + AsRawFd + 'static> DrmControlDevice for Device<T> {}

#[cfg(feature = "glutin-support")]
/// This is a horrid wrapper around [`Device`] that lets us implement Glutin's
/// `NativeWindowSource` on `Device` while allowing [`Surface`] to be generic
/// over `TS`.
///
/// [`Device`]: crate::Device
/// [`Surface`]: crate::Surface
pub struct DeviceGlutinWrapper<'a, TD: AsRawFd + 'static, TS: 'static>(&'a Device<TD>, PhantomData<TS>);

#[cfg(feature = "glutin-support")]
impl<'a, TD: AsRawFd + 'static, TS: 'static> From<&'a Device<TD>> for DeviceGlutinWrapper<'a, TD, TS> {
    fn from(d: &Device<TD>) -> DeviceGlutinWrapper<TD, TS> {
        DeviceGlutinWrapper(d, PhantomData)
    }
}

#[cfg(feature = "glutin-support")]
impl<'a, TD: AsRawFd + 'static, TS: 'static> NativeWindowSource for DeviceGlutinWrapper<'a, TD, TS> {
    type Window = Surface<TS>;
    type WindowBuilder = (PhysicalSize<u32>, BufferObjectFlags);

    fn build_wayland(
        &self,
        _wb: Self::WindowBuilder,
        _wwp: WaylandWindowParts,
    ) -> Result<Self::Window, Error> {
        unimplemented!("GBM does not provide Wayland support")
    }

    fn build_x11(&self, _wb: Self::WindowBuilder, _xwp: X11WindowParts) -> Result<Self::Window, Error> {
        unimplemented!("GBM does not provide X11 support")
    }

    fn build_gbm(&self, wb: Self::WindowBuilder, gbmwp: GbmWindowParts) -> Result<Self::Window, Error> {
        if !wb.1.contains(BufferObjectFlags::RENDERING) {
            return Err(make_error!(ErrorType::BadApiUsage(
                "BufferObjectFlags::RENDERING was not present.".to_string()
            )));
        }
        self.0
            .create_surface(
                wb.0.width,
                wb.0.height,
                Format::from_ffi(gbmwp.color_format).unwrap(),
                wb.1,
            )
            .map_err(|err| make_oserror!(OsError::IoError(Arc::new(err))))
    }
}

#[cfg(feature = "glutin-support")]
impl<T: AsRawFd + 'static> NativeDisplay for Device<T> {
    fn raw_display(&self) -> RawDisplay {
        RawDisplay::Gbm {
            gbm_device: Some(*self.ffi as *mut _),
            _non_exhaustive_do_not_use: Seal,
        }
    }
}

impl<T: AsRawFd + 'static> Drop for Device<T> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_device_destroy(*self.ffi) };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Thrown when the underlying gbm device was already destroyed
pub struct DeviceDestroyedError;

impl fmt::Display for DeviceDestroyedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "The underlying gbm device was already destroyed")
    }
}

impl error::Error for DeviceDestroyedError {}
