use {AsRaw, BufferObject, BufferObjectFlags, Format, Surface, Ptr};

use libc::c_void;

use std::error;
use std::ffi::CStr;
use std::fmt;
use std::io::{Error as IoError, Result as IoResult};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "import-wayland")]
use wayland_server::Resource;

#[cfg(feature = "import-wayland")]
use wayland_server::protocol::wl_buffer::WlBuffer;

#[cfg(feature = "import-egl")]
/// An EGLImage handle
pub type EGLImage = *mut c_void;

#[cfg(feature = "drm-support")]
use drm::Device as DrmDevice;
#[cfg(feature = "drm-support")]
use drm::control::Device as DrmControlDevice;

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
    ffi: Ptr<::ffi::gbm_device>,
}

unsafe impl Send for Ptr<::ffi::gbm_device> {}

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
    ///
    pub unsafe fn new_from_fd(fd: RawFd) -> IoResult<Device<FdWrapper>> {
        let ptr = ::ffi::gbm_create_device(fd);
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Device {
                fd: FdWrapper(fd),
                ffi: Ptr::new(ptr, |ptr| ::ffi::gbm_device_destroy(ptr)),
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
                fd: fd,
                ffi: Ptr::<::ffi::gbm_device>::new(ptr, |ptr| unsafe { ::ffi::gbm_device_destroy(ptr) }),
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
        unsafe {
            ::ffi::gbm_device_is_format_supported(
                *self.ffi,
                format.as_ffi(),
                usage.bits(),
            ) != 0
        }
    }

    /// Allocate a new surface object
    pub fn create_surface<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        usage: BufferObjectFlags,
    ) -> IoResult<Surface<U>> {
        let ptr = unsafe {
            ::ffi::gbm_surface_create(
                *self.ffi,
                width,
                height,
                format.as_ffi(),
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::new(ptr, self.ffi.downgrade()) })
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
        let ptr = unsafe {
            ::ffi::gbm_bo_create(
                *self.ffi,
                width,
                height,
                format.as_ffi(),
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.downgrade()) })
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
        buffer: &WlBuffer,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let ptr = unsafe {
            ::ffi::gbm_bo_import(
                *self.ffi,
                ::ffi::GBM_BO_IMPORT::WL_BUFFER as u32,
                buffer.ptr() as *mut _,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.downgrade()) })
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
        let ptr =
            ::ffi::gbm_bo_import(
                *self.ffi,
                ::ffi::GBM_BO_IMPORT::EGL_IMAGE as u32,
                buffer,
                usage.bits(),
            );
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(BufferObject::new(ptr, self.ffi.downgrade()))
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
            width: width,
            height: height,
            stride: stride,
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
            Ok(unsafe { BufferObject::new(ptr, self.ffi.downgrade()) })
        }
    }
}

#[cfg(feature = "drm-support")]
impl<T: DrmDevice + AsRawFd + 'static> DrmDevice for Device<T> {}

#[cfg(feature = "drm-support")]
impl<T: DrmControlDevice + AsRawFd + 'static> DrmControlDevice for Device<T> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Thrown when the underlying gbm device was already destroyed
pub struct DeviceDestroyedError;

impl fmt::Display for DeviceDestroyedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(f, "{}", self.description())
    }
}

impl error::Error for DeviceDestroyedError {
    fn description(&self) -> &str {
        "The underlying gbm device was already destroyed"
    }

    fn cause(&self) -> Option<&dyn error::Error> { None }
}
