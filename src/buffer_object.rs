use std::io::{Result as IoResult, Error as IoError};
use std::marker::PhantomData;
use std::os::unix::io::{AsRawFd, RawFd};
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::slice;

#[cfg(feature = "import_image")]
use image::{ImageBuffer, Rgba};

#[cfg(feature = "drm-support")]
use drm::buffer::{Buffer as DrmBuffer, Id as DrmId, PixelFormat as DrmPixelFormat};

use ::{AsRaw, FromRaw, Format};

/// A gbm buffer object
pub struct BufferObject<'a, T: 'static> {
    ffi: *mut ::ffi::gbm_bo,
    _lifetime: PhantomData<&'a ()>,
    _userdata: PhantomData<T>,
}

/// Flags to indicate the intended use for the buffer - these are passed into
/// `Device::create_buffer_object`.
///
/// Use `Device::is_format_supported` to check if the combination of format
/// and use flags are supported
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferObjectFlags {
    /// Buffer is going to be presented to the screen using an API such as KMS
    Scanout,
    /// Buffer is going to be used as cursor
    Cursor,
    /// Buffer is to be used for rendering - for example it is going to be used
    /// as the storage for a color buffer
    Rendering,
    /// Buffer can be used for gbm_bo_write.  This is guaranteed to work
    /// with `BufferObjectFlags::Cursor`, but may not work for other combinations.
    Write,
    /// Buffer is linear, i.e. not tiled.
    Linear,
}

impl BufferObjectFlags {
    #[doc(hidden)]
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

/// Flags to indicate the type of mapping for the buffer - these are
/// passed into `BufferObject::map()``. The caller must set the union of all the
/// flags that are appropriate.
///
/// These flags are independent of the `BufferObjectFlags` creation flags. However,
/// mapping the buffer may require copying to/from a staging buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BufferObjectTransferFlags {
    /// Buffer contents read back (or accessed directly) at transfer create time.
    Read,
    /// Buffer contents will be written back at unmap time
    /// (or modified as a result of being accessed directly)
    Write,
    /// Read/modify/write
    ReadWrite,
}

/// Abstraction representing the handle to a buffer allocated by the manager
pub type BufferObjectHandle = ::ffi::gbm_bo_handle;

/// Common functionality for all mapped buffer objects
pub trait ReadableMappedBufferObject<'a> {
    /// Get the stride of the buffer object
    ///
    /// This is calculated by the backend when it does the allocation of the buffer.
    fn stride(&self) -> u32;
    /// The X (top left origin) starting position of the mapped region for the buffer
    fn x(&self) -> u32;
    /// The Y (top left origin) starting position of the mapped region for the buffer
    fn y(&self) -> u32;
    /// The height of the mapped region for the buffer
    fn height(&self) -> u32;
    /// The width of the mapped region for the buffer
    fn width(&self) -> u32;
    /// Access to the underlying image buffer
    fn buffer(&'a self) -> &'a [u8];
}

/// Common functionality for all writable mapped buffer objects
pub trait WritableMappedBufferObject<'a>: ReadableMappedBufferObject<'a> {
    /// Mutable access to the underlying image buffer
    fn buffer_mut(&'a mut self) -> &'a mut [u8];
}

/// A read-only mapped buffer object
pub struct MappedBufferObject<'a, T: 'static> {
    bo: &'a BufferObject<'a, T>,
    addr: *mut ::libc::c_void,
    buffer: &'a mut [u8],
    stride: u32,
    height: u32,
    width: u32,
    x: u32,
    y: u32,
}

/// A read-write mapped buffer object
pub struct MappedBufferObjectRW<'a, T: 'static> {
    bo: &'a mut BufferObject<'a, T>,
    addr: *mut ::libc::c_void,
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
        unsafe { ::ffi::gbm_bo_unmap(self.bo.ffi, self.addr) }
    }
}

impl<'a, T: 'static> Drop for MappedBufferObjectRW<'a, T> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_bo_unmap(self.bo.ffi, self.addr) }
    }
}

unsafe extern fn destroy<T: 'static>(_: *mut ::ffi::gbm_bo, ptr: *mut ::libc::c_void) {
    let ptr = ptr as *mut T;
    if !ptr.is_null() { let _ = Box::from_raw(ptr); }
}

impl<'a, T: 'static> BufferObject<'a, T> {
    /// Get the width of the buffer object
    pub fn width(&self) -> u32 {
        unsafe { ::ffi::gbm_bo_get_width(self.ffi) }
    }

    /// Get the height of the buffer object
    pub fn height(&self) -> u32 {
        unsafe { ::ffi::gbm_bo_get_height(self.ffi) }
    }

    /// Get the stride of the buffer object
    pub fn stride(&self) -> u32 {
        unsafe { ::ffi::gbm_bo_get_width(self.ffi) }
    }

    /// Get the format of the buffer object
    pub fn format(&self) -> Format {
        Format::from_ffi(unsafe { ::ffi::gbm_bo_get_format(self.ffi) }).expect("libgbm returned invalid buffer format")
    }

    /// Get the handle of the buffer object
    ///
    /// This is stored in the platform generic union `BufferObjectHandle` type. However
    /// the format of this handle is platform specific.
    pub fn handle(&self) -> BufferObjectHandle {
        unsafe { ::ffi::gbm_bo_get_handle(self.ffi) }
    }

    /// Map a region of a gbm buffer object for cpu access
    ///
    /// This function maps a region of a gbm bo for cpu read access.
    pub fn map(&'a self, x: u32, y: u32, width: u32, height: u32) -> IoResult<MappedBufferObject<'a, T>> {
        unsafe {
            let mut buffer: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ::ffi::gbm_bo_map(self.ffi, x, y, width, height, ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32, &mut stride as *mut _, &mut buffer as *mut _);

            if ptr.is_null() {
                Err(IoError::last_os_error())
            } else {
                Ok(MappedBufferObject {
                    bo: self,
                    addr: ptr,
                    buffer: slice::from_raw_parts_mut(buffer as *mut _, ((height*stride+height*width)*4) as usize),
                    stride: stride,
                    height: height,
                    width: width,
                    x: x,
                    y: y,
                })
            }
        }
    }

    /// Map a region of a gbm buffer object for cpu access
    ///
    /// This function maps a region of a gbm bo for cpu read/write access.
    pub fn map_mut(&'a mut self, x: u32, y: u32, width: u32, height: u32) -> IoResult<MappedBufferObjectRW<'a, T>> {
        unsafe {
            let mut buffer: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ::ffi::gbm_bo_map(self.ffi, x, y, width, height, ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32, &mut stride as *mut _, &mut buffer as *mut _);

            if ptr.is_null() {
                Err(IoError::last_os_error())
            } else {
                Ok(MappedBufferObjectRW {
                    bo: self,
                    addr: ptr,
                    buffer: slice::from_raw_parts_mut(buffer as *mut _, ((height*stride+height*width)*4) as usize),
                    stride: stride,
                    height: height,
                    width: width,
                    x: x,
                    y: y,
                })
            }
        }
    }

    ///  Write data into the buffer object
    ///
    /// If the buffer object was created with the `BufferObjectFlags::Write` flag,
    /// this function can be used to write data into the buffer object.  The
    /// data is copied directly into the object and it's the responsibility
    /// of the caller to make sure the data represents valid pixel data,
    /// according to the width, height, stride and format of the buffer object.
    pub fn write(&mut self, buffer: &[u8]) -> IoResult<()> {
        let result = unsafe { ::ffi::gbm_bo_write(self.ffi, buffer.as_ptr() as *const _, buffer.len()) };
        if result != 0 {
            Err(IoError::last_os_error())
        } else {
            Ok(())
        }
    }

    /// Sets the userdata of the buffer object.
    ///
    /// If previously userdata was set, it is returned.
    pub fn set_userdata(&mut self, userdata: T) -> Option<T> {
        let old = self.take_userdata();

        let boxed = Box::new(userdata);
        unsafe { ::ffi::gbm_bo_set_user_data(self.ffi, Box::into_raw(boxed) as *mut _, Some(destroy::<T>)); }

        old
    }

    /// Clears the set userdata of the buffer object.
    pub fn clear_userdata(&mut self) {
        let _ = self.take_userdata();
    }

    /// Returns a reference to set userdata, if any.
    pub fn userdata(&self) -> Option<&T> {
        let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe { Some(&*(raw as *mut T)) }
        }
    }

    /// Returns a mutable reference to set userdata, if any.
    pub fn userdata_mut(&mut self) -> Option<&mut T> {
        let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe { Some(&mut *(raw as *mut T)) }
        }
    }

    /// Takes ownership of previously set userdata, if any.
    ///
    /// This removes the userdata from the buffer object.
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

#[cfg(feature = "drm-support")]
impl<'a, T: 'static> DrmBuffer for BufferObject<'a, T> {
    fn size(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    fn format(&self) -> DrmPixelFormat {
        DrmPixelFormat::from_raw(self.format().as_ffi()).unwrap()
    }

    fn pitch(&self) -> u32 {
        self.stride()
    }

    fn handle(&self) -> DrmId {
        unsafe { DrmId::from_raw(*self.handle().u32.as_ref()) }
    }
}
