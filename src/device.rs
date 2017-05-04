use std::ffi::CStr;
use std::os::unix::io::{RawFd, AsRawFd};

#[cfg(feature = "import_egl")]
use egli::egl::EGLImage;

#[cfg(feature = "import_wayland")]
use wayland_server::protocol::wl_buffer::WlBuffer;
#[cfg(feature = "import_wayland")]
use wayland_server::Resource;

use ::{AsRaw, FromRaw, Surface, BufferObject, BufferObjectFormat, BufferObjectFlags};

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
    pub unsafe fn new(fd: RawFd) -> Option<Device> {
        let ptr = ::ffi::gbm_create_device(fd);
        if ptr.is_null() {
            None
        } else {
            Some(Device {
                ffi: ptr
            })
        }
    }

    pub fn backend_name(&self) -> &str {
        unsafe { CStr::from_ptr(::ffi::gbm_device_get_backend_name(self.ffi)).to_str().expect("GBM passed invalid utf8 string") }
    }

    pub fn is_format_supported(&self, format: BufferObjectFormat, usage: &[BufferObjectFlags]) -> bool {
        unsafe { ::ffi::gbm_device_is_format_supported(self.ffi, format.as_ffi(), usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) != 0 }
    }

    pub fn create_surface<'a>(&'a mut self, width: u32, height: u32, format: BufferObjectFormat, usage: &[BufferObjectFlags]) -> Surface<'a> {
        unsafe { Surface::from_raw(::ffi::gbm_surface_create(self.ffi, width, height, format.as_ffi(), usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x)) ) }
    }

    pub fn create_buffer_object<'a, T: 'static>(&'a mut self, width: u32, height: u32, format: BufferObjectFormat, usage: &[BufferObjectFlags]) -> BufferObject<'a, T> {
        unsafe { BufferObject::from_raw(::ffi::gbm_bo_create(self.ffi, width, height, format.as_ffi(), usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x))) }
    }

    #[cfg(feature = "import_wayland")]
    pub fn import_buffer_object_from_wayland<'a, T: 'static>(&'a mut self, buffer: &WlBuffer, usage: &[BufferObjectFlags]) -> BufferObject<'a, T> {
        unsafe { BufferObject::from_raw(::ffi::gbm_bo_import(self.ffi, ::ffi::GBM_BO_IMPORT::WL_BUFFER as u32, buffer.ptr() as *mut _, usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x))) }
    }

    #[cfg(feature = "import_egl")]
    pub fn import_buffer_object_from_egl<'a, T: 'static>(&'a mut self, buffer: &EGLImage, usage: &[BufferObjectFlags]) -> BufferObject<'a, T> {
        unsafe { BufferObject::from_raw(::ffi::gbm_bo_import(self.ffi, ::ffi::GBM_BO_IMPORT::EGL_IMAGE as u32, *buffer, usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x))) }
    }

    pub fn import_buffer_object_from_dma_buf<'a, T: 'static>(&'a mut self, buffer: RawFd, width: u32, height: u32, stride: u32, format: BufferObjectFormat, usage: &[BufferObjectFlags]) -> BufferObject<'a, T> {
        let mut fd_data = ::ffi::gbm_import_fd_data {
            fd: buffer,
            width: width,
            height: height,
            stride: stride,
            format: format.as_ffi(),
        };

        unsafe { BufferObject::from_raw(::ffi::gbm_bo_import(self.ffi, ::ffi::GBM_BO_IMPORT::FD as u32, &mut fd_data as *mut ::ffi::gbm_import_fd_data as *mut _, usage.iter().map(|x| x.as_ffi()).fold(0u32, |flag, x| flag | x))) }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_device_destroy(self.ffi) };
    }
}
