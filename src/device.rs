use {AsRaw, BufferObject, BufferObjectFlags, Format, FromRaw, Surface};

#[cfg(feature = "import-egl")]
use egli::egl::EGLImage;
use std::ffi::CStr;
use std::marker::PhantomData;
use std::io::{Error as IoError, Result as IoResult};
use std::os::unix::io::{AsRawFd, IntoRawFd, RawFd};

#[cfg(feature = "drm-support")]
use drm::Device as DrmDevice;

#[cfg(feature = "import-wayland")]
use wayland_server::Resource;

#[cfg(feature = "import-wayland")]
use wayland_server::protocol::wl_buffer::WlBuffer;

/// An open DRM device
pub struct Device<'a> {
    ffi: *mut ::ffi::gbm_device,
    _lifetime: PhantomData<&'a ()>,
}

impl<'a> AsRawFd for Device<'a> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { ::ffi::gbm_device_get_fd(self.ffi) }
    }
}

impl<'a> AsRaw<::ffi::gbm_device> for Device<'a> {
    fn as_raw(&self) -> *const ::ffi::gbm_device {
        self.ffi
    }
}

impl<'a> FromRaw<::ffi::gbm_device> for Device<'a> {
    unsafe fn from_raw(ffi: *mut ::ffi::gbm_device) -> Self {
        Device { ffi: ffi, _lifetime: PhantomData }
    }
}

impl<'a> Device<'a> {
    /// Open a GBM device from a given IO object taking ownership
    pub fn new<I: IntoRawFd>(io: I) -> IoResult<Device<'static>> {
        unsafe { Device::new_from_fd(io.into_raw_fd()) }
    }

    /// Open a GBM device from a given DRM device
    #[cfg(feature = "drm-support")]
    pub fn new_from_drm<D: DrmDevice + AsRawFd + 'a>(drm: &'a D) -> IoResult<Device<'a>> {
        unsafe { Device::new_from_fd(drm.as_raw_fd()) }
    }

    /// Open a GBM device from a given unix file descriptor.
    ///
    /// The file descriptor passed in is used by the backend to communicate with
    /// platform for allocating the memory. For allocations using DRI this would be
    /// the file descriptor returned when opening a device such as /dev/dri/card0.
    ///
    /// # Unsafety
    ///
    /// The lifetime of the resulting device depends on the ownership of the file descriptor.
    /// If the fd will be controlled by the device a static lifetime is valid, if it does not own the fd
    /// the lifetime may not outlive the owning object.
    ///
    pub unsafe fn new_from_fd(fd: RawFd) -> IoResult<Device<'a>> {
        let ptr = ::ffi::gbm_create_device(fd);
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Device { ffi: ptr, _lifetime: PhantomData })
        }
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &str {
        unsafe {
            CStr::from_ptr(::ffi::gbm_device_get_backend_name(self.ffi))
                .to_str()
                .expect("GBM passed invalid utf8 string")
        }
    }

    /// Test if a format is supported for a given set of usage flags
    pub fn is_format_supported(&self, format: Format, usage: &[BufferObjectFlags]) -> bool {
        unsafe {
            ::ffi::gbm_device_is_format_supported(
                self.ffi,
                format.as_ffi(),
                usage.iter().map(|x| x.as_ffi()).fold(
                    0u32,
                    |flag, x| flag | x,
                ),
            ) != 0
        }
    }

    /// Allocate a new surface object
    pub fn create_surface<T: 'static>(
        &'a self,
        width: u32,
        height: u32,
        format: Format,
        usage: &[BufferObjectFlags],
    ) -> IoResult<Surface<'a, T>> {
        let ptr = unsafe {
            ::ffi::gbm_surface_create(
                self.ffi,
                width,
                height,
                format.as_ffi(),
                usage.iter().map(|x| x.as_ffi()).fold(
                    0u32,
                    |flag, x| flag | x,
                ),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::from_raw(ptr) })
        }
    }

    ///  Allocate a buffer object for the given dimensions
    pub fn create_buffer_object<T: 'static>(
        &'a self,
        width: u32,
        height: u32,
        format: Format,
        usage: &[BufferObjectFlags],
    ) -> IoResult<BufferObject<'a, T>> {
        let ptr = unsafe {
            ::ffi::gbm_bo_create(
                self.ffi,
                width,
                height,
                format.as_ffi(),
                usage.iter().map(|x| x.as_ffi()).fold(
                    0u32,
                    |flag, x| flag | x,
                ),
            )
        };
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
    #[cfg(feature = "import-wayland")]
    pub fn import_buffer_object_from_wayland<T: 'static>(
        &'a self,
        buffer: &WlBuffer,
        usage: &[BufferObjectFlags],
    ) -> IoResult<BufferObject<'a, T>> {
        let ptr = unsafe {
            ::ffi::gbm_bo_import(
                self.ffi,
                ::ffi::GBM_BO_IMPORT::WL_BUFFER as u32,
                buffer.ptr() as *mut _,
                usage.iter().map(|x| x.as_ffi()).fold(
                    0u32,
                    |flag, x| flag | x,
                ),
            )
        };
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
    #[cfg(feature = "import-egl")]
    pub fn import_buffer_object_from_egl<T: 'static>(
        &'a self,
        buffer: &EGLImage,
        usage: &[BufferObjectFlags],
    ) -> IoResult<BufferObject<'a, T>> {
        let ptr = unsafe {
            ::ffi::gbm_bo_import(
                self.ffi,
                ::ffi::GBM_BO_IMPORT::EGL_IMAGE as u32,
                *buffer,
                usage.iter().map(|x| x.as_ffi()).fold(
                    0u32,
                    |flag, x| flag | x,
                ),
            )
        };
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
    pub fn import_buffer_object_from_dma_buf<T: 'static>(
        &'a self,
        buffer: RawFd,
        width: u32,
        height: u32,
        stride: u32,
        format: Format,
        usage: &[BufferObjectFlags],
    ) -> IoResult<BufferObject<'a, T>> {
        let mut fd_data = ::ffi::gbm_import_fd_data {
            fd: buffer,
            width: width,
            height: height,
            stride: stride,
            format: format.as_ffi(),
        };

        let ptr = unsafe {
            ::ffi::gbm_bo_import(
                self.ffi,
                ::ffi::GBM_BO_IMPORT::FD as u32,
                &mut fd_data as *mut ::ffi::gbm_import_fd_data as *mut _,
                usage.iter().map(|x| x.as_ffi()).fold(
                    0u32,
                    |flag, x| flag | x,
                ),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::from_raw(ptr) })
        }
    }
}

impl<'a> Drop for Device<'a> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_device_destroy(self.ffi) };
    }
}
