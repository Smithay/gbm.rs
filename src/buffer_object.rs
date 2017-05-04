use std::marker::PhantomData;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::slice;

#[cfg(feature = "import_image")]
use image::{ImageBuffer, Rgba};

use ::{AsRaw, FromRaw};

pub struct BufferObject<'a, T: 'static> {
    ffi: *mut ::ffi::gbm_bo,
    _lifetime: PhantomData<&'a ()>,
    _userdata: PhantomData<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferObjectFormat {
    XRGB8888,
    ARGB8888,
}

impl BufferObjectFormat {
    pub fn as_ffi(&self) -> u32 {
        match *self {
            BufferObjectFormat::XRGB8888 => ::ffi::gbm_bo_format::GBM_BO_FORMAT_XRGB8888 as u32,
            BufferObjectFormat::ARGB8888 => ::ffi::gbm_bo_format::GBM_BO_FORMAT_ARGB8888 as u32,
        }
    }

    pub fn from_ffi(raw: u32) -> Option<BufferObjectFormat> {
        match raw {
            x if x == ::ffi::gbm_bo_format::GBM_BO_FORMAT_XRGB8888 as u32 => Some(BufferObjectFormat::XRGB8888),
            x if x == ::ffi::gbm_bo_format::GBM_BO_FORMAT_ARGB8888 as u32 => Some(BufferObjectFormat::ARGB8888),
            _ => None
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferObjectFlags {
    Scanout,
    Cursor,
    Rendering,
    Write,
    Linear,
}

impl BufferObjectFlags {
    pub fn as_ffi(&self) -> u32 {
        match *self {
            BufferObjectFlags::Scanout => ::ffi::gbm_bo_flags_GBM_BO_USE_SCANOUT as u32,
            BufferObjectFlags::Cursor => ::ffi::gbm_bo_flags_GBM_BO_USE_CURSOR as u32,
            BufferObjectFlags::Rendering => ::ffi::gbm_bo_flags_GBM_BO_USE_RENDERING as u32,
            BufferObjectFlags::Write => ::ffi::gbm_bo_flags_GBM_BO_USE_WRITE as u32,
            BufferObjectFlags::Linear => ::ffi::gbm_bo_flags_GBM_BO_USE_LINEAR as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferObjectTransferFlags {
    Read,
    Write,
    ReadWrite,
}

pub type BufferObjectHandle = ::ffi::gbm_bo_handle;

pub enum InvalidBufferError {
    InsuffientSize,
}

pub trait ReadableMappedBufferObject<'a> {
    fn stride(&self) -> u32;
    fn x(&self) -> u32;
    fn y(&self) -> u32;
    fn height(&self) -> u32;
    fn width(&self) -> u32;
    fn buffer(&'a self) -> &'a [u8];
}

pub trait WritableMappedBufferObject<'a>: ReadableMappedBufferObject<'a> {
    fn buffer_mut(&'a mut self) -> &'a mut [u8];
}

pub struct MappedBufferObject<'a, T: 'static> {
    bo: &'a BufferObject<'a, T>,
    buffer: &'a mut [u8],
    stride: u32,
    height: u32,
    width: u32,
    x: u32,
    y: u32,
}

pub struct MappedBufferObjectRW<'a, T: 'static> {
    bo: &'a mut BufferObject<'a, T>,
    buffer: &'a mut [u8],
    stride: u32,
    height: u32,
    width: u32,
    x: u32,
    y: u32,
}

impl<'a, T: 'static> Deref for MappedBufferObject<'a, T> {
    type Target = BufferObject<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.bo
    }
}

impl<'a, T: 'static> Deref for MappedBufferObjectRW<'a, T> {
    type Target = BufferObject<'a, T>;

    fn deref(&self) -> &Self::Target {
        &self.bo
    }
}

impl<'a, T: 'static> DerefMut for MappedBufferObjectRW<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.bo
    }
}

impl<'a, T: 'static> ReadableMappedBufferObject<'a> for MappedBufferObject<'a, T> {
    fn stride(&self) -> u32 {
        self.stride
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn x(&self) -> u32 {
        self.x
    }

    fn y(&self) -> u32 {
        self.y
    }

    fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }
}

impl<'a, T: 'static> ReadableMappedBufferObject<'a> for MappedBufferObjectRW<'a, T> {
    fn stride(&self) -> u32 {
        self.stride
    }

    fn height(&self) -> u32 {
        self.height
    }

    fn width(&self) -> u32 {
        self.width
    }

    fn x(&self) -> u32 {
        self.x
    }

    fn y(&self) -> u32 {
        self.y
    }

    fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }
}

impl<'a, T: 'static> WritableMappedBufferObject<'a> for MappedBufferObjectRW<'a, T> {
    fn buffer_mut(&'a mut self) -> &'a mut [u8] {
        self.buffer
    }
}

impl<'a, T: 'static> Drop for MappedBufferObject<'a, T> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_bo_unmap(self.bo.ffi, self.buffer as *mut [u8] as *mut _) }
    }
}

impl<'a, T: 'static> Drop for MappedBufferObjectRW<'a, T> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_bo_unmap(self.bo.ffi, self.buffer as *mut [u8] as *mut _) }
    }
}

unsafe extern fn destroy<T: 'static>(_: *mut ::ffi::gbm_bo, ptr: *mut ::libc::c_void) {
    let ptr = ptr as *mut T;
    if !ptr.is_null() { let _ = Box::from_raw(ptr); }
}

impl<'a, T: 'static> BufferObject<'a, T> {
    pub fn width(&self) -> u32 {
        unsafe { ::ffi::gbm_bo_get_width(self.ffi) }
    }

    pub fn height(&self) -> u32 {
        unsafe { ::ffi::gbm_bo_get_height(self.ffi) }
    }

    pub fn stride(&self) -> u32 {
        unsafe { ::ffi::gbm_bo_get_width(self.ffi) }
    }

    pub fn format(&self) -> BufferObjectFormat {
        BufferObjectFormat::from_ffi(unsafe { ::ffi::gbm_bo_get_format(self.ffi) }).expect("libgbm returned invalid buffer format")
    }

    pub fn handle(&self) -> BufferObjectHandle {
        unsafe { ::ffi::gbm_bo_get_handle(self.ffi) }
    }

    pub fn map(&'a self, x: u32, y: u32, width: u32, height: u32) -> MappedBufferObject<'a, T> {
        unsafe {
            let mut buffer: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            ::ffi::gbm_bo_map(self.ffi, x, y, width, height, ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32, &mut stride as *mut _, &mut buffer as *mut _);

            MappedBufferObject {
                bo: self,
                buffer: slice::from_raw_parts_mut(buffer as *mut _, ((height*stride+height*width)*4) as usize),
                stride: stride,
                height: height,
                width: width,
                x: x,
                y: y,
            }
        }
    }

    pub fn map_mut(&'a mut self, x: u32, y: u32, width: u32, height: u32) -> MappedBufferObjectRW<'a, T> {
        unsafe {
            let mut buffer: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            ::ffi::gbm_bo_map(self.ffi, x, y, width, height, ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32, &mut stride as *mut _, &mut buffer as *mut _);

            MappedBufferObjectRW {
                bo: self,
                buffer: slice::from_raw_parts_mut(buffer as *mut _, ((height*stride+height*width)*4) as usize),
                stride: stride,
                height: height,
                width: width,
                x: x,
                y: y,
            }
        }
    }

    pub fn write(&mut self, buffer: &[u8]) -> Result<(), InvalidBufferError> {
        let size = self.height() * self.width() + self.height() * self.stride();

        if buffer.len() < size as usize {
            Err(InvalidBufferError::InsuffientSize)
        } else {
            Ok(())
        }
    }

    pub fn set_userdata(&mut self, userdata: T) -> Option<T> {
        let old = self.take_userdata();

        let boxed = Box::new(userdata);
        unsafe { ::ffi::gbm_bo_set_user_data(self.ffi, Box::into_raw(boxed) as *mut _, Some(destroy::<T>)); }

        old
    }

    pub fn clear_userdata(&mut self) {
        let _ = self.take_userdata();
    }

    pub fn userdata(&self) -> Option<&T> {
        let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe { Some(&*(raw as *mut T)) }
        }
    }

    pub fn userdata_mut(&mut self) -> Option<&mut T> {
        let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe { Some(&mut *(raw as *mut T)) }
        }
    }

    pub fn take_userdata(&mut self) -> Option<T> {
        let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe {
                let boxed = Box::from_raw(raw as *mut T);
                ::ffi::gbm_bo_set_user_data(self.ffi, ptr::null_mut(), None);
                Some(*boxed)
            }
        }
    }
}

impl<'a, T: 'static> AsRawFd for BufferObject<'a, T> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { ::ffi::gbm_bo_get_fd(self.ffi) }
    }
}

impl<'a, T: 'static> AsRaw<::ffi::gbm_bo> for BufferObject<'a, T> {
    fn as_raw(&self) -> *const ::ffi::gbm_bo {
        self.ffi
    }
}

impl<'a, T: 'static> FromRaw<::ffi::gbm_bo> for BufferObject<'a, T> {
    unsafe fn from_raw(ffi: *mut ::ffi::gbm_bo) -> Self {
        BufferObject {
            ffi: ffi,
            _lifetime: PhantomData,
            _userdata: PhantomData,
        }
    }
}

impl<'a, T: 'static> Drop for BufferObject<'a, T> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_bo_destroy(self.ffi) }
    }
}
