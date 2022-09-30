use {AsRaw, Device, DeviceDestroyedError, Format, Modifier, Ptr, WeakPtr};

#[cfg(feature = "drm-support")]
use drm::buffer::{Buffer as DrmBuffer, Handle, PlanarBuffer as DrmPlanarBuffer};

use std::error;
use std::fmt;
use std::io::{Error as IoError, Result as IoResult};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::os::unix::io::{AsRawFd, RawFd};
use std::ptr;
use std::slice;

/// A GBM buffer object
pub struct BufferObject<T: 'static> {
    pub(crate) ffi: Ptr<ffi::gbm_bo>,
    pub(crate) _device: WeakPtr<::ffi::gbm_device>,
    pub(crate) _userdata: PhantomData<T>,
}

impl<T> fmt::Debug for BufferObject<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BufferObject")
            .field("ptr", &format_args!("{:p}", self.ffi))
            .field("device", &format_args!("{:p}", &self._device))
            .field("width", &self.width().unwrap_or(0))
            .field("height", &self.height().unwrap_or(0))
            .field("offsets", &self.offsets())
            .field("stride", &self.stride().unwrap_or(0))
            .field("format", &self.format().ok())
            .field("modifier", &self.modifier().ok())
            .finish()
    }
}

bitflags! {
    /// Flags to indicate the intended use for the buffer - these are passed into
    /// [`Device::create_buffer_object()`].
    ///
    /// Use [`Device::is_format_supported()`] to check if the combination of format
    /// and use flags are supported
    pub struct BufferObjectFlags: u32 {
        /// Buffer is going to be presented to the screen using an API such as KMS
        const SCANOUT      = ::ffi::gbm_bo_flags::GBM_BO_USE_SCANOUT as u32;
        /// Buffer is going to be used as cursor
        const CURSOR       = ::ffi::gbm_bo_flags::GBM_BO_USE_CURSOR as u32;
        /// Buffer is going to be used as cursor (deprecated)
        #[deprecated = "Use CURSOR instead"]
        const CURSOR_64X64 = ::ffi::gbm_bo_flags::GBM_BO_USE_CURSOR_64X64 as u32;
        /// Buffer is to be used for rendering - for example it is going to be used
        /// as the storage for a color buffer
        const RENDERING    = ::ffi::gbm_bo_flags::GBM_BO_USE_RENDERING as u32;
        /// Buffer can be used for [`BufferObject::write()`].  This is guaranteed to work
        /// with [`Self::CURSOR`], but may not work for other combinations.
        const WRITE        = ::ffi::gbm_bo_flags::GBM_BO_USE_WRITE as u32;
        /// Buffer is linear, i.e. not tiled.
        const LINEAR       = ::ffi::gbm_bo_flags::GBM_BO_USE_LINEAR as u32;
        /// Buffer is protected
        const PROTECTED    = ::ffi::gbm_bo_flags::GBM_BO_USE_PROTECTED as u32;
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
    buffer: &'a mut [u8],
    data: *mut ::libc::c_void,
    stride: u32,
    height: u32,
    width: u32,
    x: u32,
    y: u32,
}

impl<'a, T> fmt::Debug for MappedBufferObject<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("MappedBufferObject")
            .field(
                "mode",
                &match self.bo {
                    BORef::Ref(_) => format_args!("read"),
                    BORef::Mut(_) => format_args!("write"),
                },
            )
            .field(
                "buffer",
                match &self.bo {
                    BORef::Ref(bo) => *bo,
                    BORef::Mut(bo) => *bo,
                },
            )
            .finish()
    }
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
    pub fn buffer(&self) -> &[u8] {
        self.buffer
    }

    /// Mutable access to the underlying image buffer
    pub fn buffer_mut(&mut self) -> &mut [u8] {
        self.buffer
    }
}

impl<'a, T: 'static> Deref for MappedBufferObject<'a, T> {
    type Target = BufferObject<T>;
    fn deref(&self) -> &BufferObject<T> {
        match &self.bo {
            BORef::Ref(bo) => bo,
            BORef::Mut(bo) => bo,
        }
    }
}

impl<'a, T: 'static> DerefMut for MappedBufferObject<'a, T> {
    fn deref_mut(&mut self) -> &mut BufferObject<T> {
        match &mut self.bo {
            BORef::Ref(_) => unreachable!(),
            BORef::Mut(bo) => bo,
        }
    }
}

impl<'a, T: 'static> Drop for MappedBufferObject<'a, T> {
    fn drop(&mut self) {
        let ffi = match &self.bo {
            BORef::Ref(bo) => &bo.ffi,
            BORef::Mut(bo) => &bo.ffi,
        };
        unsafe { ::ffi::gbm_bo_unmap(**ffi, self.data) }
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
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_width(*self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the height of the buffer object
    pub fn height(&self) -> Result<u32, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_height(*self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the stride of the buffer object
    pub fn stride(&self) -> Result<u32, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_stride(*self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the stride of the buffer object
    pub fn stride_for_plane(&self, plane: i32) -> Result<u32, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_stride_for_plane(*self.ffi, plane) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the format of the buffer object
    pub fn format(&self) -> Result<Format, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            use std::convert::TryFrom;
            Ok(
                Format::try_from(unsafe { ::ffi::gbm_bo_get_format(*self.ffi) })
                    .expect("libgbm returned invalid buffer format"),
            )
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the bits per pixel of the buffer object
    pub fn bpp(&self) -> Result<u32, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_bpp(*self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the offset for a plane of the buffer object
    pub fn offset(&self, plane: i32) -> Result<u32, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_offset(*self.ffi, plane) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the plane count of the buffer object
    pub fn plane_count(&self) -> Result<u32, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_plane_count(*self.ffi) as u32 })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the modifier of the buffer object
    pub fn modifier(&self) -> Result<Modifier, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(Modifier::from(unsafe {
                ::ffi::gbm_bo_get_modifier(*self.ffi)
            }))
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get a DMA-BUF file descriptor for the buffer object
    ///
    /// This function creates a DMA-BUF (also known as PRIME) file descriptor
    /// handle for the buffer object.  Each call to [`Self::fd()`] returns a new
    /// file descriptor and the caller is responsible for closing the file
    /// descriptor.
    pub fn fd(&self) -> Result<RawFd, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_fd(*self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get the handle of the buffer object
    ///
    /// This is stored in the platform generic union [`BufferObjectHandle`] type.  However
    /// the format of this handle is platform specific.
    pub fn handle(&self) -> Result<BufferObjectHandle, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_handle(*self.ffi) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Get a DMA-BUF file descriptor for a plane of the buffer object
    ///
    /// This function creates a DMA-BUF (also known as PRIME) file descriptor
    /// handle for a plane of the buffer object. Each call to [`Self::fd_for_plane()`]
    /// returns a new file descriptor and the caller is responsible for closing
    /// the file descriptor.
    #[cfg(any(HAS_GBM_BO_GET_FD_FOR_PLANE, not(feature = "auto-detect")))]
    pub fn fd_for_plane(&self, plane: i32) -> Result<RawFd, PlaneFdError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_fd_for_plane(*self.ffi, plane) })
        } else {
            Err(PlaneFdError::Destroyed(DeviceDestroyedError))
        }
    }

    /// Get a DMA-BUF file descriptor for a plane of the buffer object
    ///
    /// This function creates a DMA-BUF (also known as PRIME) file descriptor
    /// handle for a plane of the buffer object. Each call to [`Self::fd_for_plane()`]
    /// returns a new file descriptor and the caller is responsible for closing
    /// the file descriptor.
    #[cfg(all(not(HAS_GBM_BO_GET_FD_FOR_PLANE), feature = "auto-detect"))]
    pub fn fd_for_plane(&self, plane: i32) -> Result<RawFd, PlaneFdError> {
        Err(PlaneFdError::Unsupported)
    }

    /// Get the handle of a plane of the buffer object
    ///
    /// This is stored in the platform generic union [`BufferObjectHandle`] type.  However
    /// the format of this handle is platform specific.
    pub fn handle_for_plane(&self, plane: i32) -> Result<BufferObjectHandle, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            Ok(unsafe { ::ffi::gbm_bo_get_handle_for_plane(*self.ffi, plane) })
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Map a region of a GBM buffer object for cpu access
    ///
    /// This function maps a region of a GBM bo for cpu read access.
    pub fn map<'a, D, F, S>(
        &'a self,
        device: &Device<D>,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        f: F,
    ) -> Result<IoResult<S>, WrongDeviceError>
    where
        D: AsRawFd + 'static,
        F: FnOnce(&MappedBufferObject<'a, T>) -> S,
    {
        let device_ref = self._device.upgrade();
        if let Some(_device) = device_ref {
            if *_device != device.as_raw_mut() {
                // not matching
                return Err(WrongDeviceError);
            }
        } else {
            // not matching
            return Err(WrongDeviceError);
        }

        unsafe {
            let mut data: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ::ffi::gbm_bo_map(
                *self.ffi,
                x,
                y,
                width,
                height,
                ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32,
                &mut stride as *mut _,
                &mut data as *mut _,
            );

            if ptr.is_null() {
                Ok(Err(IoError::last_os_error()))
            } else {
                Ok(Ok(f(&MappedBufferObject {
                    bo: BORef::Ref(self),
                    buffer: slice::from_raw_parts_mut(ptr as *mut _, (height * stride) as usize),
                    data,
                    stride,
                    height,
                    width,
                    x,
                    y,
                })))
            }
        }
    }

    /// Map a region of a GBM buffer object for cpu access
    ///
    /// This function maps a region of a GBM bo for cpu read/write access.
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
        let device_ref = self._device.upgrade();
        if let Some(_device) = device_ref {
            if *_device != device.as_raw_mut() {
                // not matching
                return Err(WrongDeviceError);
            }
        } else {
            // not matching
            return Err(WrongDeviceError);
        }

        unsafe {
            let mut data: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ::ffi::gbm_bo_map(
                *self.ffi,
                x,
                y,
                width,
                height,
                ::ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ_WRITE as u32,
                &mut stride as *mut _,
                &mut data as *mut _,
            );

            if ptr.is_null() {
                Ok(Err(IoError::last_os_error()))
            } else {
                Ok(Ok(f(&mut MappedBufferObject {
                    bo: BORef::Mut(self),
                    buffer: slice::from_raw_parts_mut(ptr as *mut _, (height * stride) as usize),
                    data,
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
    /// If the buffer object was created with the [`BufferObjectFlags::WRITE`] flag,
    /// this function can be used to write data into the buffer object.  The
    /// data is copied directly into the object and it's the responsibility
    /// of the caller to make sure the data represents valid pixel data,
    /// according to the width, height, stride and format of the buffer object.
    pub fn write(&mut self, buffer: &[u8]) -> Result<IoResult<()>, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            let result = unsafe {
                ::ffi::gbm_bo_write(*self.ffi, buffer.as_ptr() as *const _, buffer.len() as _)
            };
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
        let device = self._device.upgrade();
        if device.is_some() {
            let old = self.take_userdata();

            let boxed = Box::new(userdata);
            unsafe {
                ::ffi::gbm_bo_set_user_data(
                    *self.ffi,
                    Box::into_raw(boxed) as *mut _,
                    Some(destroy::<T>),
                );
            }

            old
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Clears the set userdata of the buffer object.
    pub fn clear_userdata(&mut self) -> Result<(), DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            let _ = self.take_userdata();
            Ok(())
        } else {
            Err(DeviceDestroyedError)
        }
    }

    /// Returns a reference to set userdata, if any.
    pub fn userdata(&self) -> Result<Option<&T>, DeviceDestroyedError> {
        let device = self._device.upgrade();
        if device.is_some() {
            let raw = unsafe { ::ffi::gbm_bo_get_user_data(*self.ffi) };

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
        let device = self._device.upgrade();
        if device.is_some() {
            let raw = unsafe { ::ffi::gbm_bo_get_user_data(*self.ffi) };

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
        let device = self._device.upgrade();
        if device.is_some() {
            let raw = unsafe { ::ffi::gbm_bo_get_user_data(*self.ffi) };

            if raw.is_null() {
                Ok(None)
            } else {
                unsafe {
                    let boxed = Box::from_raw(raw as *mut T);
                    ::ffi::gbm_bo_set_user_data(*self.ffi, ptr::null_mut(), None);
                    Ok(Some(*boxed))
                }
            }
        } else {
            Err(DeviceDestroyedError)
        }
    }

    pub(crate) unsafe fn new(
        ffi: *mut ::ffi::gbm_bo,
        device: WeakPtr<::ffi::gbm_device>,
    ) -> BufferObject<T> {
        BufferObject {
            ffi: Ptr::<::ffi::gbm_bo>::new(ffi, |ptr| ::ffi::gbm_bo_destroy(ptr)),
            _device: device,
            _userdata: PhantomData,
        }
    }
}

impl<T: 'static> AsRawFd for BufferObject<T> {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { ::ffi::gbm_bo_get_fd(*self.ffi) }
    }
}

impl<T: 'static> AsRaw<::ffi::gbm_bo> for BufferObject<T> {
    fn as_raw(&self) -> *const ::ffi::gbm_bo {
        *self.ffi
    }
}

#[cfg(feature = "drm-support")]
impl<T: 'static> DrmBuffer for BufferObject<T> {
    fn size(&self) -> (u32, u32) {
        (
            self.width().expect("GbmDevice does not exist anymore"),
            self.height().expect("GbmDevice does not exist anymore"),
        )
    }

    fn format(&self) -> Format {
        BufferObject::<T>::format(self).expect("GbmDevice does not exist anymore")
    }

    fn pitch(&self) -> u32 {
        self.stride().expect("GbmDevice does not exist anymore")
    }

    fn handle(&self) -> Handle {
        use std::num::NonZeroU32;
        unsafe {
            Handle::from(NonZeroU32::new_unchecked(
                self.handle()
                    .expect("GbmDevice does not exist anymore")
                    .u32_,
            ))
        }
    }
}

#[cfg(feature = "drm-support")]
impl<T: 'static> DrmPlanarBuffer for BufferObject<T> {
    fn size(&self) -> (u32, u32) {
        (
            self.width().expect("GbmDevice does not exist anymore"),
            self.height().expect("GbmDevice does not exist anymore"),
        )
    }
    fn format(&self) -> Format {
        BufferObject::<T>::format(self).expect("GbmDevice does not exist anymore")
    }
    fn pitches(&self) -> [u32; 4] {
        let num = self
            .plane_count()
            .expect("GbmDevice does not exist anymore");
        [
            BufferObject::<T>::stride_for_plane(self, 0).unwrap(),
            if num > 1 {
                BufferObject::<T>::stride_for_plane(self, 1).unwrap()
            } else {
                0
            },
            if num > 2 {
                BufferObject::<T>::stride_for_plane(self, 2).unwrap()
            } else {
                0
            },
            if num > 3 {
                BufferObject::<T>::stride_for_plane(self, 3).unwrap()
            } else {
                0
            },
        ]
    }
    fn handles(&self) -> [Option<Handle>; 4] {
        use std::num::NonZeroU32;
        let num = self
            .plane_count()
            .expect("GbmDevice does not exist anymore");
        [
            Some(unsafe {
                Handle::from(NonZeroU32::new_unchecked(
                    BufferObject::<T>::handle_for_plane(self, 0).unwrap().u32_,
                ))
            }),
            if num > 1 {
                Some(unsafe {
                    Handle::from(NonZeroU32::new_unchecked(
                        BufferObject::<T>::handle_for_plane(self, 1).unwrap().u32_,
                    ))
                })
            } else {
                None
            },
            if num > 2 {
                Some(unsafe {
                    Handle::from(NonZeroU32::new_unchecked(
                        BufferObject::<T>::handle_for_plane(self, 2).unwrap().u32_,
                    ))
                })
            } else {
                None
            },
            if num > 3 {
                Some(unsafe {
                    Handle::from(NonZeroU32::new_unchecked(
                        BufferObject::<T>::handle_for_plane(self, 3).unwrap().u32_,
                    ))
                })
            } else {
                None
            },
        ]
    }
    fn offsets(&self) -> [u32; 4] {
        let num = self
            .plane_count()
            .expect("GbmDevice does not exist anymore");
        [
            BufferObject::<T>::offset(self, 0).unwrap(),
            if num > 1 {
                BufferObject::<T>::offset(self, 1).unwrap()
            } else {
                0
            },
            if num > 2 {
                BufferObject::<T>::offset(self, 2).unwrap()
            } else {
                0
            },
            if num > 3 {
                BufferObject::<T>::offset(self, 3).unwrap()
            } else {
                0
            },
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Thrown when the GBM device does not belong to the buffer object
pub struct WrongDeviceError;

impl fmt::Display for WrongDeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "The gbm device specified is not the one this buffer object belongs to"
        )
    }
}

impl error::Error for WrongDeviceError {
    fn cause(&self) -> Option<&dyn error::Error> {
        None
    }
}

/// Error for [`BufferObject::fd_for_plane`]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaneFdError {
    /// Thrown when the underlying GBM device was already destroyed
    Destroyed(DeviceDestroyedError),
    /// Thrown when the function is not supported on this platform
    Unsupported,
}

impl fmt::Display for PlaneFdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PlaneFdError::Destroyed(e) => fmt::Display::fmt(e, f),
            PlaneFdError::Unsupported => {
                write!(f, "The gbm device does not support fd_for_plane")
            }
        }
    }
}

impl error::Error for PlaneFdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PlaneFdError::Destroyed(e) => e.source(),
            PlaneFdError::Unsupported => None,
        }
    }
}
