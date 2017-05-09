use std::ffi::CStr;
use std::io::{Result as IoResult, Error as IoError};
use std::os::unix::io::{RawFd, AsRawFd};

#[cfg(feature = "import_egl")]
use egli::egl::EGLImage;

#[cfg(feature = "import_wayland")]
use wayland_server::protocol::wl_buffer::WlBuffer;
#[cfg(feature = "import_wayland")]
use wayland_server::Resource;

use ::{AsRaw, FromRaw, Surface, BufferObject, Format, BufferObjectFlags};

/// An open DRM device
pub struct Device {
    ffi: *mut ::ffi::gbm_device,
}

impl AsRawFd for Device {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { ::ffi::gbm_device_get_fd(self.ffi) }
    }
}

impl AsRaw<::ffi::gbm_device> for Device {
    fn as_raw(&self) -> *const ::ffi::gbm_device {
        self.ffi
    }
}

impl FromRaw<::ffi::gbm_device> for Device {
    unsafe fn from_raw(ffi: *mut ::ffi::gbm_device) -> Self {
        Device {
            ffi: ffi,
        }
    }
}

impl Device {
    /// Open a DRM device from a given unix file descriptor.
    ///
    /// The file descriptor passed in is used by the backend to communicate with
    /// platform for allocating the memory. For allocations using DRI this would be
    /// the file descriptor returned when opening a device such as /dev/dri/card0.
    ///
    /// # Unsafety
    ///
    /// If the file descriptor was not created from a usable device behavior is
    /// not defined.
    ///
    pub unsafe fn new(fd: RawFd) -> IoResult<Device> {
        let ptr = ::ffi::gbm_create_device(fd);
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Device { ffi: ptr })
        }
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &str {
        unsafe { CStr::from_ptr(::ffi::gbm_device_get_backend_name(self.ffi)).to_str().expect("GBM passed invalid utf8 string") }
    }

    /// Test if a format is supported for a given set of usage flags
    pub fn is_format_supported(&self, format: Format, usage: &[BufferObjectFlags]) -> bool {
        unsafe { ::ffi::gbm_device_is_format_supported(self.ffi, format.as_ffi(), usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) != 0 }
    }

    /// Allocate a new surface object
    pub fn create_surface<'a>(&'a mut self, width: u32, height: u32, format: Format, usage: &[BufferObjectFlags]) -> IoResult<Surface<'a>> {
        let ptr = unsafe { ::ffi::gbm_surface_create(self.ffi, width, height, format.as_ffi(), usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::from_raw(ptr) })
        }
    }

    ///  Allocate a buffer object for the given dimensions
    pub fn create_buffer_object<'a, T: 'static>(&'a mut self, width: u32, height: u32, format: Format, usage: &[BufferObjectFlags]) -> IoResult<BufferObject<'a, T>> {
        let ptr = unsafe { ::ffi::gbm_bo_create(self.ffi, width, height, format.as_ffi(), usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::from_raw(ptr) })
        }
    }

    ///  Create a gbm buffer object from a wayland buffer
    ///
    /// This function imports a foreign `WlBuffer` object and creates a new gbm
    /// buffer object for it.
    /// This enabled using the foreign object with a display API such as KMS.
    ///
    /// The gbm bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    #[cfg(feature = "import_wayland")]
    pub fn import_buffer_object_from_wayland<'a, T: 'static>(&'a mut self, buffer: &WlBuffer, usage: &[BufferObjectFlags]) -> IoResult<BufferObject<'a, T>> {
        let ptr = unsafe { ::ffi::gbm_bo_import(self.ffi, ::ffi::GBM_BO_IMPORT::WL_BUFFER as u32, buffer.ptr() as *mut _, usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::from_raw(ptr) })
        }
    }

    ///  Create a gbm buffer object from an egl buffer
    ///
    /// This function imports a foreign `EGLImage` object and creates a new gbm
    /// buffer object for it.
    /// This enabled using the foreign object with a display API such as KMS.
    ///
    /// The gbm bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    #[cfg(feature = "import_egl")]
    pub fn import_buffer_object_from_egl<'a, T: 'static>(&'a mut self, buffer: &EGLImage, usage: &[BufferObjectFlags]) -> IoResult<BufferObject<'a, T>> {
        let ptr = unsafe { ::ffi::gbm_bo_import(self.ffi, ::ffi::GBM_BO_IMPORT::EGL_IMAGE as u32, *buffer, usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::from_raw(ptr) })
        }
    }

    ///  Create a gbm buffer object from an dma buffer
    ///
    /// This function imports a foreign dma buffer from an open file descriptor
    /// and creates a new gbm buffer object for it.
    /// This enabled using the foreign object with a display API such as KMS.
    ///
    /// The gbm bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    pub fn import_buffer_object_from_dma_buf<'a, T: 'static>(&'a mut self, buffer: RawFd, width: u32, height: u32, stride: u32, format: Format, usage: &[BufferObjectFlags]) -> IoResult<BufferObject<'a, T>> {
        let mut fd_data = ::ffi::gbm_import_fd_data {
            fd: buffer,
            width: width,
            height: height,
            stride: stride,
            format: format.as_ffi(),
        };

        let ptr = unsafe { ::ffi::gbm_bo_import(self.ffi, ::ffi::GBM_BO_IMPORT::FD as u32, &mut fd_data as *mut ::ffi::gbm_import_fd_data as *mut _, usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::from_raw(ptr) })
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_device_destroy(self.ffi) };
    }
}
