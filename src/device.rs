use crate::{AsRaw, BufferObject, BufferObjectFlags, Format, Modifier, Ptr, Surface};

use std::os::unix::io::{AsFd, AsRawFd, BorrowedFd};

use std::ffi::CStr;
use std::fmt;
use std::io::{Error as IoError, Result as IoResult};
use std::ops::{Deref, DerefMut};

#[cfg(feature = "import-wayland")]
use wayland_server::protocol::wl_buffer::WlBuffer;

#[cfg(feature = "import-egl")]
/// An EGLImage handle
pub type EGLImage = *mut libc::c_void;

#[cfg(feature = "drm-support")]
use drm::control::Device as DrmControlDevice;
#[cfg(feature = "drm-support")]
use drm::Device as DrmDevice;

/// An open GBM device
pub struct Device<T: AsFd> {
    // Declare `ffi` first so it is dropped before `fd`
    ffi: Ptr<ffi::gbm_device>,
    fd: T,
}

impl<T: AsFd> fmt::Debug for Device<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Device")
            .field("ptr", &format_args!("{:p}", &self.ffi))
            .finish()
    }
}

impl<T: AsFd + Clone> Clone for Device<T> {
    fn clone(&self) -> Device<T> {
        Device {
            fd: self.fd.clone(),
            ffi: self.ffi.clone(),
        }
    }
}

impl<T: AsFd> AsFd for Device<T> {
    fn as_fd(&self) -> BorrowedFd {
        unsafe { BorrowedFd::borrow_raw(ffi::gbm_device_get_fd(*self.ffi)) }
    }
}

impl<T: AsFd> AsRaw<ffi::gbm_device> for Device<T> {
    fn as_raw(&self) -> *const ffi::gbm_device {
        *self.ffi
    }
}

impl<T: AsFd> Deref for Device<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.fd
    }
}

impl<T: AsFd> DerefMut for Device<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.fd
    }
}

impl<T: AsFd> Device<T> {
    /// Open a GBM device from a given open DRM device.
    ///
    /// The underlying file descriptor passed in is used by the backend to communicate with
    /// platform for allocating the memory.  For allocations using DRI this would be
    /// the file descriptor returned when opening a device such as `/dev/dri/card0`.
    pub fn new(fd: T) -> IoResult<Device<T>> {
        let ptr = unsafe { ffi::gbm_create_device(fd.as_fd().as_raw_fd()) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(Device {
                fd,
                ffi: Ptr::<ffi::gbm_device>::new(ptr, |ptr| unsafe {
                    ffi::gbm_device_destroy(ptr)
                }),
            })
        }
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &str {
        unsafe {
            CStr::from_ptr(ffi::gbm_device_get_backend_name(*self.ffi))
                .to_str()
                .expect("GBM passed invalid utf8 string")
        }
    }

    /// Test if a format is supported for a given set of usage flags
    pub fn is_format_supported(&self, format: Format, usage: BufferObjectFlags) -> bool {
        unsafe { ffi::gbm_device_is_format_supported(*self.ffi, format as u32, usage.bits()) != 0 }
    }

    /// Get the required number of planes for a given format and modifier
    ///
    /// Some combination (e.g. when using a `Modifier::Invalid`) might not
    /// have a defined/fixed number of planes. In these cases the function
    /// might return `Option::None`.
    pub fn format_modifier_plane_count(&self, format: Format, modifier: Modifier) -> Option<u32> {
        unsafe {
            ffi::gbm_device_get_format_modifier_plane_count(
                *self.ffi,
                format as u32,
                modifier.into(),
            )
            .try_into()
            .ok()
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
            ffi::gbm_surface_create(*self.ffi, width, height, format as u32, usage.bits())
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::new(ptr, self.ffi.clone()) })
        }
    }

    /// Allocate a new surface object with explicit modifiers
    pub fn create_surface_with_modifiers<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        modifiers: impl Iterator<Item = Modifier>,
    ) -> IoResult<Surface<U>> {
        let mods = modifiers.map(|m| m.into()).collect::<Vec<u64>>();
        let ptr = unsafe {
            ffi::gbm_surface_create_with_modifiers(
                *self.ffi,
                width,
                height,
                format as u32,
                mods.as_ptr(),
                mods.len() as u32,
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::new(ptr, self.ffi.clone()) })
        }
    }

    /// Allocate a new surface object with explicit modifiers and flags
    pub fn create_surface_with_modifiers2<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        modifiers: impl Iterator<Item = Modifier>,
        usage: BufferObjectFlags,
    ) -> IoResult<Surface<U>> {
        let mods = modifiers.map(|m| m.into()).collect::<Vec<u64>>();
        let ptr = unsafe {
            ffi::gbm_surface_create_with_modifiers2(
                *self.ffi,
                width,
                height,
                format as u32,
                mods.as_ptr(),
                mods.len() as u32,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { Surface::new(ptr, self.ffi.clone()) })
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
        let ptr =
            unsafe { ffi::gbm_bo_create(*self.ffi, width, height, format as u32, usage.bits()) };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.clone()) })
        }
    }

    ///  Allocate a buffer object for the given dimensions with explicit modifiers
    pub fn create_buffer_object_with_modifiers<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        modifiers: impl Iterator<Item = Modifier>,
    ) -> IoResult<BufferObject<U>> {
        let mods = modifiers.map(|m| m.into()).collect::<Vec<u64>>();
        let ptr = unsafe {
            ffi::gbm_bo_create_with_modifiers(
                *self.ffi,
                width,
                height,
                format as u32,
                mods.as_ptr(),
                mods.len() as u32,
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.clone()) })
        }
    }

    ///  Allocate a buffer object for the given dimensions with explicit modifiers and flags
    pub fn create_buffer_object_with_modifiers2<U: 'static>(
        &self,
        width: u32,
        height: u32,
        format: Format,
        modifiers: impl Iterator<Item = Modifier>,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let mods = modifiers.map(|m| m.into()).collect::<Vec<u64>>();
        let ptr = unsafe {
            ffi::gbm_bo_create_with_modifiers2(
                *self.ffi,
                width,
                height,
                format as u32,
                mods.as_ptr(),
                mods.len() as u32,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.clone()) })
        }
    }

    /// Create a GBM buffer object from a wayland buffer
    ///
    /// This function imports a foreign [`WlBuffer`] object and creates a new GBM
    /// buffer object for it.
    /// This enables using the foreign object with a display API such as KMS.
    ///
    /// The GBM bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    #[cfg(feature = "import-wayland")]
    pub fn import_buffer_object_from_wayland<U: 'static>(
        &self,
        buffer: &WlBuffer,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        use wayland_server::Resource;

        let ptr = unsafe {
            ffi::gbm_bo_import(
                *self.ffi,
                ffi::GBM_BO_IMPORT_WL_BUFFER,
                buffer.id().as_ptr() as *mut _,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.clone()) })
        }
    }

    /// Create a GBM buffer object from an egl buffer
    ///
    /// This function imports a foreign [`EGLImage`] object and creates a new GBM
    /// buffer object for it.
    /// This enables using the foreign object with a display API such as KMS.
    ///
    /// The GBM bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    ///
    /// # Safety
    ///
    /// The given [`EGLImage`] is a raw pointer.  Passing null or an invalid [`EGLImage`] will
    /// cause undefined behavior.
    #[cfg(feature = "import-egl")]
    pub unsafe fn import_buffer_object_from_egl<U: 'static>(
        &self,
        buffer: EGLImage,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let ptr = ffi::gbm_bo_import(
            *self.ffi,
            ffi::GBM_BO_IMPORT_EGL_IMAGE,
            buffer,
            usage.bits(),
        );
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(BufferObject::new(ptr, self.ffi.clone()))
        }
    }

    /// Create a GBM buffer object from a dma buffer
    ///
    /// This function imports a foreign dma buffer from an open file descriptor
    /// and creates a new GBM buffer object for it.
    /// This enables using the foreign object with a display API such as KMS.
    ///
    /// The GBM bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    pub fn import_buffer_object_from_dma_buf<U: 'static>(
        &self,
        buffer: BorrowedFd<'_>,
        width: u32,
        height: u32,
        stride: u32,
        format: Format,
        usage: BufferObjectFlags,
    ) -> IoResult<BufferObject<U>> {
        let mut fd_data = ffi::gbm_import_fd_data {
            fd: buffer.as_raw_fd(),
            width,
            height,
            stride,
            format: format as u32,
        };

        let ptr = unsafe {
            ffi::gbm_bo_import(
                *self.ffi,
                ffi::GBM_BO_IMPORT_FD,
                &mut fd_data as *mut ffi::gbm_import_fd_data as *mut _,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.clone()) })
        }
    }

    /// Create a GBM buffer object from a dma buffer with explicit modifiers
    ///
    /// This function imports a foreign dma buffer from an open file descriptor
    /// and creates a new GBM buffer object for it.
    /// This enables using the foreign object with a display API such as KMS.
    ///
    /// The GBM bo shares the underlying pixels but its life-time is
    /// independent of the foreign object.
    #[allow(clippy::too_many_arguments)]
    pub fn import_buffer_object_from_dma_buf_with_modifiers<U: 'static>(
        &self,
        len: u32,
        buffers: [Option<BorrowedFd<'_>>; 4],
        width: u32,
        height: u32,
        format: Format,
        usage: BufferObjectFlags,
        strides: [i32; 4],
        offsets: [i32; 4],
        modifier: Modifier,
    ) -> IoResult<BufferObject<U>> {
        let fds = buffers.map(|fd| fd.map_or(-1, |x| x.as_raw_fd()));
        let mut fd_data = ffi::gbm_import_fd_modifier_data {
            fds,
            width,
            height,
            format: format as u32,
            strides,
            offsets,
            modifier: modifier.into(),
            num_fds: len,
        };

        let ptr = unsafe {
            ffi::gbm_bo_import(
                *self.ffi,
                ffi::GBM_BO_IMPORT_FD_MODIFIER,
                &mut fd_data as *mut ffi::gbm_import_fd_modifier_data as *mut _,
                usage.bits(),
            )
        };
        if ptr.is_null() {
            Err(IoError::last_os_error())
        } else {
            Ok(unsafe { BufferObject::new(ptr, self.ffi.clone()) })
        }
    }
}

#[cfg(feature = "drm-support")]
impl<T: DrmDevice + AsFd> DrmDevice for Device<T> {}

#[cfg(feature = "drm-support")]
impl<T: DrmControlDevice + AsFd> DrmControlDevice for Device<T> {}
