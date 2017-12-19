use {AsRaw, Device, DeviceDestroyedError, Format};

#[cfg(feature = "drm-support")]
use drm::buffer::{Buffer as DrmBuffer, Id as DrmId, PixelFormat as DrmPixelFormat};

use std::error;
use std::fmt;
use std::io::{Error as IoError, Result as IoResult};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::os::unix::io::{AsRawFd, RawFd};
use std::rc::Weak;
use std::ptr;
use std::slice;

/// A gbm buffer object
pub struct BufferObject<T: 'static> {
    ffi: *mut ::ffi::gbm_bo,
    pub(crate) _device: Weak<*mut ::ffi::gbm_device>,
    _userdata: PhantomData<T>,
}

bitflags! {
    /// Flags to indicate the intended use for the buffer - these are passed into
    /// `Device::create_buffer_object`.
    ///
    /// Use `Device::is_format_supported` to check if the combination of format
    /// and use flags are supported
    pub struct BufferObjectFlags: u32 {
        /// Buffer is going to be presented to the screen using an API such as KMS
        const SCANOUT      = ::ffi::gbm_bo_flags_GBM_BO_USE_SCANOUT as u32;
        /// Buffer is going to be used as cursor
        const CURSOR       = ::ffi::gbm_bo_flags_GBM_BO_USE_CURSOR as u32;
        /// Buffer is to be used for rendering - for example it is going to be used
        /// as the storage for a color buffer
        const RENDERING    = ::ffi::gbm_bo_flags_GBM_BO_USE_RENDERING as u32;
        /// Buffer can be used for gbm_bo_write.  This is guaranteed to work
        /// with `BufferObjectFlags::Cursor`, but may not work for other combinations.
        const WRITE        = ::ffi::gbm_bo_flags_GBM_BO_USE_WRITE as u32;
        /// Buffer is linear, i.e. not tiled.
        const LINEAR       = ::ffi::gbm_bo_flags_GBM_BO_USE_LINEAR as u32;
    }
}

/// Abstraction representing the handle to a buffer allocated by the manager
pub type BufferObjectHandle = ::ffi::gbm_bo_handle;

enum BORef<'a, T: 'static> {
    Ref(&'a BufferObject<T>),
    Mut(&'a mut BufferObject<T>),
}

/// A mapped buffer object
pub struct MappedBufferObject<'a, T: 'static> {
    bo: BORef<'a, T>,
    addr: *mut ::libc::c_void,
    buffer: &'a mut [u8],
    stride: u32,
    height: u32,
    width: u32,
    x: u32,
    y: u32,
}

impl<'a, T: 'static> MappedBufferObject<'a, T> {
    /// Get the stride of the buffer object
    ///
    /// This is calculated by the backend when it does the allocation of the buffer.
    pub fn stride(&self) -> u32 {
        self.stride
    }

    /// The height of the mapped region for the buffer
    pub fn height(&self) -> u32 {
        self.height
    }

    /// The width of the mapped region for the buffer
    pub fn width(&self) -> u32 {
        self.width
    }

    /// The X (top left origin) starting position of the mapped region for the buffer
    pub fn x(&self) -> u32 {
        self.x
    }

    /// The Y (top left origin) starting position of the mapped region for the buffer
    pub fn y(&self) -> u32 {
        self.y
    }

    /// Access to the underlying image buffer
    pub fn buffer(&'a self) -> &'a [u8] {
        self.buffer
    }

    /// Mutable access to the underlying image buffer
    pub fn buffer_mut(&'a mut self) -> &'a mut [u8] {
        self.buffer
    }
}

impl<'a, T: 'static> Deref for MappedBufferObject<'a, T> {
    type Target = BufferObject<T>;
    fn deref(&self) -> &BufferObject<T> {
        match &self.bo {
            &BORef::Ref(bo) => bo,
            &BORef::Mut(ref bo) => bo,
        }
    }
}

impl<'a, T: 'static> DerefMut for MappedBufferObject<'a, T> {
    fn deref_mut(&mut self) -> &mut BufferObject<T> {
        match &mut self.bo {
            &mut BORef::Ref(_) => unreachable!(),
            &mut BORef::Mut(ref mut bo) => bo,
        }
    }
}

impl<'a, T: 'static> Drop for MappedBufferObject<'a, T> {
    fn drop(&mut self) {
        let ffi = match &self.bo {
            &BORef::Ref(bo) => bo.ffi,
            &BORef::Mut(ref bo) => bo.ffi,
        };
        unsafe { ::ffi::gbm_bo_unmap(ffi, self.addr) }
    }
}

unsafe extern "C" fn destroy<T: 'static>(_: *mut ::ffi::gbm_bo, ptr: *mut ::libc::c_void) {
    let ptr = ptr as *mut T;
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

impl<T: 'static> BufferObject<T> {
    /// Get the width of the buffer object
    pub fn width(&self) -> Result<u32, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_width(self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the height of the buffer object
    pub fn height(&self) -> Result<u32, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_height(self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the stride of the buffer object
    pub fn stride(&self) -> Result<u32, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_stride(self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the format of the buffer object
    pub fn format(&self) -> Result<Format, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            Ok(Format::from_ffi(unsafe { ::ffi::gbm_bo_get_format(self.ffi) })
            .expect("libgbm returned invalid buffer format"))
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the handle of the buffer object
    ///
    /// This is stored in the platform generic union `BufferObjectHandle` type. However
    /// the format of this handle is platform specific.
    pub fn handle(&self) -> Result<BufferObjectHandle, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_handle(self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Map a region of a gbm buffer object for cpu access
    ///
    /// This function maps a region of a gbm bo for cpu read access.
    pub fn map<'a, D, F, S>(&'a self, device: &Device<D>, x: u32, y: u32, width: u32, height: u32, f: F) -> Result<IoResult<S>, WrongDeviceError>
        where
            D: AsRawFd + 'static,
            F: FnOnce(&MappedBufferObject<'a, T>) -> S,
    {
        if let Some(_device) = self._device.upgrade() {
            if *_device != device.as_raw_mut() { // not matching
                return Err(WrongDeviceError);
            }
        } else { // not matching
            return Err(WrongDeviceError);
        }

        unsafe {
            let mut buffer: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ::ffi::gbm_bo_map(
                self.ffi,
                x,
                y,
                width,
                height,
                ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32,
                &mut stride as *mut _,
                &mut buffer as *mut _,
            );

            if ptr.is_null() {
                Ok(Err(IoError::last_os_error()))
            } else {
                Ok(Ok(f(&MappedBufferObject {
                    bo: BORef::Ref(self),
                    addr: ptr,
                    buffer: slice::from_raw_parts_mut(
                        buffer as *mut _,
                        ((height * stride + height * width) * 4) as usize,
                    ),
                    stride,
                    height,
                    width,
                    x,
                    y,
                })))
            }
        }
    }

    /// Map a region of a gbm buffer object for cpu access
    ///
    /// This function maps a region of a gbm bo for cpu read/write access.
    pub fn map_mut<'a, D, F, S>(
        &'a mut self,
        device: &Device<D>,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        f: F,
    ) -> Result<IoResult<S>, WrongDeviceError>
        where
            D: AsRawFd + 'static,
            F: FnOnce(&mut MappedBufferObject<'a, T>) -> S,
    {
        if let Some(_device) = self._device.upgrade() {
            if *_device != device.as_raw_mut() { // not matching
                return Err(WrongDeviceError);
            }
        } else { // not matching
            return Err(WrongDeviceError);
        }

        unsafe {
            let mut buffer: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ::ffi::gbm_bo_map(
                self.ffi,
                x,
                y,
                width,
                height,
                ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ_WRITE as u32,
                &mut stride as *mut _,
                &mut buffer as *mut _,
            );

            if ptr.is_null() {
                Ok(Err(IoError::last_os_error()))
            } else {
                Ok(Ok(f(&mut MappedBufferObject {
                    bo: BORef::Mut(self),
                    addr: ptr,
                    buffer: slice::from_raw_parts_mut(
                        buffer as *mut _,
                        ((height * stride + height * width) * 4) as usize,
                    ),
                    stride,
                    height,
                    width,
                    x,
                    y,
                })))
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
    pub fn write(&mut self, buffer: &[u8]) -> Result<IoResult<()>, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            let result = unsafe { ::ffi::gbm_bo_write(self.ffi, buffer.as_ptr() as *const _, buffer.len()) };
            if result != 0 {
                Ok(Err(IoError::last_os_error()))
            } else {
                Ok(Ok(()))
            }
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Sets the userdata of the buffer object.
    ///
    /// If previously userdata was set, it is returned.
    pub fn set_userdata(&mut self, userdata: T) -> Result<Option<T>, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            let old = self.take_userdata();

            let boxed = Box::new(userdata);
            unsafe {
                ::ffi::gbm_bo_set_user_data(self.ffi, Box::into_raw(boxed) as *mut _, Some(destroy::<T>));
            }

            old
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Clears the set userdata of the buffer object.
    pub fn clear_userdata(&mut self) -> Result<(), DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            let _ = self.take_userdata();
            Ok(())
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Returns a reference to set userdata, if any.
    pub fn userdata(&self) -> Result<Option<&T>, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

            if raw.is_null() {
                Ok(None)
            } else {
                unsafe { Ok(Some(&*(raw as *mut T))) }
            }
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Returns a mutable reference to set userdata, if any.
    pub fn userdata_mut(&mut self) -> Result<Option<&mut T>, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

            if raw.is_null() {
                Ok(None)
            } else {
                unsafe { Ok(Some(&mut *(raw as *mut T))) }
            }
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Takes ownership of previously set userdata, if any.
    ///
    /// This removes the userdata from the buffer object.
    pub fn take_userdata(&mut self) -> Result<Option<T>, DeviceDestroyedError> {
        if self._device.upgrade().is_some() {
            let raw = unsafe { ::ffi::gbm_bo_get_user_data(self.ffi) };

            if raw.is_null() {
                Ok(None)
            } else {
                unsafe {
                    let boxed = Box::from_raw(raw as *mut T);
                    ::ffi::gbm_bo_set_user_data(self.ffi, ptr::null_mut(), None);
                    Ok(Some(*boxed))
                }
            }
        } else {
            Err(DeviceDestroyedError)
        }
    }

    pub(crate) unsafe fn new(ffi: *mut ::ffi::gbm_bo, device: Weak<*mut ::ffi::gbm_device>) -> BufferObject<T> {
        BufferObject {
            ffi,
            _device: device,
            _userdata: PhantomData,
        }
    }
}

impl<T: 'static> AsRawFd for BufferObject<T> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { ::ffi::gbm_bo_get_fd(self.ffi) }
    }
}

impl<T: 'static> AsRaw<::ffi::gbm_bo> for BufferObject<T> {
    fn as_raw(&self) -> *const ::ffi::gbm_bo {
        self.ffi
    }
}

impl<T: 'static> Drop for BufferObject<T> {
    fn drop(&mut self) {
        if self._device.upgrade().is_some() {
            unsafe { ::ffi::gbm_bo_destroy(self.ffi) }
        }
    }
}

#[cfg(feature = "drm-support")]
impl<T: 'static> DrmBuffer for BufferObject<T> {
    fn size(&self) -> (u32, u32) {
        (self.width().expect("GbmDevice does not exist anymore"), self.height().expect("GbmDevice does not exist anymore"))
    }

    fn format(&self) -> DrmPixelFormat {
        DrmPixelFormat::from_raw(self.format().expect("GbmDevice does not exist anymore").as_ffi()).unwrap()
    }

    fn pitch(&self) -> u32 {
        self.stride().expect("GbmDevice does not exist anymore")
    }

    fn handle(&self) -> DrmId {
        unsafe { DrmId::from_raw(*self.handle().expect("GbmDevice does not exist anymore").u32.as_ref()) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Thrown when the gbm device does not belong to the buffer object
pub struct WrongDeviceError;

impl fmt::Display for WrongDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::error::Error;
        write!(f, "{}", self.description())
    }
}

impl error::Error for WrongDeviceError {
    fn description(&self) -> &str {
        "The gbm specified is not the one this buffer object belongs to"
    }

    fn cause(&self) -> Option<&error::Error> { None }
}
