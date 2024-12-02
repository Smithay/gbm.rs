#![allow(clippy::unnecessary_cast)]

use crate::{AsRaw, Format, Modifier, Ptr};

#[cfg(feature = "drm-support")]
use drm::buffer::{Buffer as DrmBuffer, Handle, PlanarBuffer as DrmPlanarBuffer};
use std::os::unix::io::{BorrowedFd, FromRawFd, OwnedFd};

use std::error;
use std::fmt;
use std::io::{Error as IoError, Result as IoResult};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::ptr;
use std::slice;

/// A GBM buffer object
pub struct BufferObject<T: 'static> {
    // Declare `ffi` first so it is dropped before `_device`
    pub(crate) ffi: Ptr<ffi::gbm_bo>,
    pub(crate) _device: Ptr<ffi::gbm_device>,
    pub(crate) _userdata: PhantomData<T>,
}

impl<T> fmt::Debug for BufferObject<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BufferObject")
            .field("ptr", &format_args!("{:p}", self.ffi))
            .field("device", &format_args!("{:p}", &self._device))
            .field("width", &self.width())
            .field("height", &self.height())
            .field("offsets", &self.offsets())
            .field("stride", &self.stride())
            .field("format", &self.format())
            .field("modifier", &self.modifier())
            .finish()
    }
}

bitflags! {
    /// Flags to indicate the intended use for the buffer - these are passed into
    /// [`Device::create_buffer_object()`].
    ///
    /// Use [`Device::is_format_supported()`] to check if the combination of format
    /// and use flags are supported
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(transparent))]
    #[derive(Hash, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
    pub struct BufferObjectFlags: u32 {
        /// Buffer is going to be presented to the screen using an API such as KMS
        const SCANOUT      = ffi::gbm_bo_flags::GBM_BO_USE_SCANOUT as u32;
        /// Buffer is going to be used as cursor
        const CURSOR       = ffi::gbm_bo_flags::GBM_BO_USE_CURSOR as u32;
        /// Buffer is going to be used as cursor (deprecated)
        #[deprecated = "Use CURSOR instead"]
        const CURSOR_64X64 = ffi::gbm_bo_flags::GBM_BO_USE_CURSOR_64X64 as u32;
        /// Buffer is to be used for rendering - for example it is going to be used
        /// as the storage for a color buffer
        const RENDERING    = ffi::gbm_bo_flags::GBM_BO_USE_RENDERING as u32;
        /// Buffer can be used for [`BufferObject::write()`].  This is guaranteed to work
        /// with [`Self::CURSOR`], but may not work for other combinations.
        const WRITE        = ffi::gbm_bo_flags::GBM_BO_USE_WRITE as u32;
        /// Buffer is linear, i.e. not tiled.
        const LINEAR       = ffi::gbm_bo_flags::GBM_BO_USE_LINEAR as u32;
        /// Buffer is protected
        const PROTECTED    = ffi::gbm_bo_flags::GBM_BO_USE_PROTECTED as u32;
    }
}

/// Abstraction representing the handle to a buffer allocated by the manager
pub type BufferObjectHandle = ffi::gbm_bo_handle;

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
        unsafe { ffi::gbm_bo_unmap(**ffi, self.data) }
    }
}

unsafe extern "C" fn destroy<T: 'static>(_: *mut ffi::gbm_bo, ptr: *mut ::libc::c_void) {
    let ptr = ptr as *mut T;
    if !ptr.is_null() {
        let _ = Box::from_raw(ptr);
    }
}

impl<T: 'static> BufferObject<T> {
    /// Get the width of the buffer object
    pub fn width(&self) -> u32 {
        unsafe { ffi::gbm_bo_get_width(*self.ffi) }
    }

    /// Get the height of the buffer object
    pub fn height(&self) -> u32 {
        unsafe { ffi::gbm_bo_get_height(*self.ffi) }
    }

    /// Get the stride of the buffer object
    pub fn stride(&self) -> u32 {
        unsafe { ffi::gbm_bo_get_stride(*self.ffi) }
    }

    /// Get the stride of the buffer object
    pub fn stride_for_plane(&self, plane: i32) -> u32 {
        unsafe { ffi::gbm_bo_get_stride_for_plane(*self.ffi, plane) }
    }

    /// Get the format of the buffer object
    pub fn format(&self) -> Format {
        Format::try_from(unsafe { ffi::gbm_bo_get_format(*self.ffi) })
            .expect("libgbm returned invalid buffer format")
    }

    /// Get the bits per pixel of the buffer object
    pub fn bpp(&self) -> u32 {
        unsafe { ffi::gbm_bo_get_bpp(*self.ffi) }
    }

    /// Get the offset for a plane of the buffer object
    pub fn offset(&self, plane: i32) -> u32 {
        unsafe { ffi::gbm_bo_get_offset(*self.ffi, plane) }
    }

    /// Get the plane count of the buffer object
    pub fn plane_count(&self) -> u32 {
        unsafe { ffi::gbm_bo_get_plane_count(*self.ffi) as u32 }
    }

    /// Get the modifier of the buffer object
    pub fn modifier(&self) -> Modifier {
        Modifier::from(unsafe { ffi::gbm_bo_get_modifier(*self.ffi) })
    }

    /// Get a DMA-BUF file descriptor for the buffer object
    ///
    /// This function creates a DMA-BUF (also known as PRIME) file descriptor
    /// handle for the buffer object.  Each call to [`Self::fd()`] returns a new
    /// file descriptor and the caller is responsible for closing the file
    /// descriptor.
    pub fn fd(&self) -> Result<OwnedFd, InvalidFdError> {
        unsafe {
            let fd = ffi::gbm_bo_get_fd(*self.ffi);

            if fd == -1 {
                return Err(InvalidFdError);
            }

            Ok(OwnedFd::from_raw_fd(fd))
        }
    }

    /// Get the file descriptor of the gbm device of this buffer object
    pub fn device_fd(&self) -> BorrowedFd {
        unsafe { BorrowedFd::borrow_raw(ffi::gbm_device_get_fd(*self._device)) }
    }

    /// Get the handle of the buffer object
    ///
    /// This is stored in the platform generic union [`BufferObjectHandle`] type.  However
    /// the format of this handle is platform specific.
    pub fn handle(&self) -> BufferObjectHandle {
        unsafe { ffi::gbm_bo_get_handle(*self.ffi) }
    }

    /// Get a DMA-BUF file descriptor for a plane of the buffer object
    ///
    /// This function creates a DMA-BUF (also known as PRIME) file descriptor
    /// handle for a plane of the buffer object. Each call to [`Self::fd_for_plane()`]
    /// returns a new file descriptor and the caller is responsible for closing
    /// the file descriptor.
    pub fn fd_for_plane(&self, plane: i32) -> Result<OwnedFd, InvalidFdError> {
        unsafe {
            let fd = ffi::gbm_bo_get_fd_for_plane(*self.ffi, plane);

            if fd == -1 {
                return Err(InvalidFdError);
            }

            Ok(OwnedFd::from_raw_fd(fd))
        }
    }

    /// Get the handle of a plane of the buffer object
    ///
    /// This is stored in the platform generic union [`BufferObjectHandle`] type.  However
    /// the format of this handle is platform specific.
    pub fn handle_for_plane(&self, plane: i32) -> BufferObjectHandle {
        unsafe { ffi::gbm_bo_get_handle_for_plane(*self.ffi, plane) }
    }

    /// Map a region of a GBM buffer object for cpu access
    ///
    /// This function maps a region of a GBM bo for cpu read access.
    pub fn map<'a, F, S>(&'a self, x: u32, y: u32, width: u32, height: u32, f: F) -> IoResult<S>
    where
        F: FnOnce(&MappedBufferObject<'a, T>) -> S,
    {
        unsafe {
            let mut data: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ffi::gbm_bo_map(
                *self.ffi,
                x,
                y,
                width,
                height,
                ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ as u32,
                &mut stride as *mut _,
                &mut data as *mut _,
            );

            if ptr.is_null() {
                Err(IoError::last_os_error())
            } else {
                Ok(f(&MappedBufferObject {
                    bo: BORef::Ref(self),
                    buffer: slice::from_raw_parts_mut(ptr as *mut _, (height * stride) as usize),
                    data,
                    stride,
                    height,
                    width,
                    x,
                    y,
                }))
            }
        }
    }

    /// Map a region of a GBM buffer object for cpu access
    ///
    /// This function maps a region of a GBM bo for cpu read/write access.
    pub fn map_mut<'a, F, S>(
        &'a mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        f: F,
    ) -> IoResult<S>
    where
        F: FnOnce(&mut MappedBufferObject<'a, T>) -> S,
    {
        unsafe {
            let mut data: *mut ::libc::c_void = ptr::null_mut();
            let mut stride = 0;
            let ptr = ffi::gbm_bo_map(
                *self.ffi,
                x,
                y,
                width,
                height,
                ffi::gbm_bo_transfer_flags::GBM_BO_TRANSFER_READ_WRITE as u32,
                &mut stride as *mut _,
                &mut data as *mut _,
            );

            if ptr.is_null() {
                Err(IoError::last_os_error())
            } else {
                Ok(f(&mut MappedBufferObject {
                    bo: BORef::Mut(self),
                    buffer: slice::from_raw_parts_mut(ptr as *mut _, (height * stride) as usize),
                    data,
                    stride,
                    height,
                    width,
                    x,
                    y,
                }))
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
    pub fn write(&mut self, buffer: &[u8]) -> IoResult<()> {
        let result =
            unsafe { ffi::gbm_bo_write(*self.ffi, buffer.as_ptr() as *const _, buffer.len() as _) };
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
        unsafe {
            ffi::gbm_bo_set_user_data(
                *self.ffi,
                Box::into_raw(boxed) as *mut _,
                Some(destroy::<T>),
            );
        }

        old
    }

    /// Clears the set userdata of the buffer object.
    pub fn clear_userdata(&mut self) {
        let _ = self.take_userdata();
    }

    /// Returns a reference to set userdata, if any.
    pub fn userdata(&self) -> Option<&T> {
        let raw = unsafe { ffi::gbm_bo_get_user_data(*self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe { Some(&*(raw as *mut T)) }
        }
    }

    /// Returns a mutable reference to set userdata, if any.
    pub fn userdata_mut(&mut self) -> Option<&mut T> {
        let raw = unsafe { ffi::gbm_bo_get_user_data(*self.ffi) };

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
        let raw = unsafe { ffi::gbm_bo_get_user_data(*self.ffi) };

        if raw.is_null() {
            None
        } else {
            unsafe {
                let boxed = Box::from_raw(raw as *mut T);
                ffi::gbm_bo_set_user_data(*self.ffi, ptr::null_mut(), None);
                Some(*boxed)
            }
        }
    }

    pub(crate) unsafe fn new(
        ffi: *mut ffi::gbm_bo,
        device: Ptr<ffi::gbm_device>,
    ) -> BufferObject<T> {
        BufferObject {
            ffi: Ptr::<ffi::gbm_bo>::new(ffi, |ptr| ffi::gbm_bo_destroy(ptr)),
            _device: device,
            _userdata: PhantomData,
        }
    }

    fn offsets(&self) -> [u32; 4] {
        let num = self.plane_count();
        [
            BufferObject::<T>::offset(self, 0),
            if num > 1 {
                BufferObject::<T>::offset(self, 1)
            } else {
                0
            },
            if num > 2 {
                BufferObject::<T>::offset(self, 2)
            } else {
                0
            },
            if num > 3 {
                BufferObject::<T>::offset(self, 3)
            } else {
                0
            },
        ]
    }
}

impl<T: 'static> AsRaw<ffi::gbm_bo> for BufferObject<T> {
    fn as_raw(&self) -> *const ffi::gbm_bo {
        *self.ffi
    }
}

#[cfg(feature = "drm-support")]
impl<T: 'static> DrmBuffer for BufferObject<T> {
    fn size(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    fn format(&self) -> Format {
        BufferObject::<T>::format(self)
    }

    fn pitch(&self) -> u32 {
        self.stride()
    }

    fn handle(&self) -> Handle {
        use std::num::NonZeroU32;
        unsafe { Handle::from(NonZeroU32::new_unchecked(self.handle().u32_)) }
    }
}

#[cfg(feature = "drm-support")]
impl<T: 'static> DrmPlanarBuffer for BufferObject<T> {
    fn size(&self) -> (u32, u32) {
        (self.width(), self.height())
    }
    fn format(&self) -> Format {
        BufferObject::<T>::format(self)
    }
    fn modifier(&self) -> Option<Modifier> {
        Some(BufferObject::<T>::modifier(self))
    }
    fn pitches(&self) -> [u32; 4] {
        let num = self.plane_count();
        [
            BufferObject::<T>::stride_for_plane(self, 0),
            if num > 1 {
                BufferObject::<T>::stride_for_plane(self, 1)
            } else {
                0
            },
            if num > 2 {
                BufferObject::<T>::stride_for_plane(self, 2)
            } else {
                0
            },
            if num > 3 {
                BufferObject::<T>::stride_for_plane(self, 3)
            } else {
                0
            },
        ]
    }
    fn handles(&self) -> [Option<Handle>; 4] {
        use std::num::NonZeroU32;
        let num = self.plane_count();
        [
            Some(unsafe {
                Handle::from(NonZeroU32::new_unchecked(
                    BufferObject::<T>::handle_for_plane(self, 0).u32_,
                ))
            }),
            if num > 1 {
                Some(unsafe {
                    Handle::from(NonZeroU32::new_unchecked(
                        BufferObject::<T>::handle_for_plane(self, 1).u32_,
                    ))
                })
            } else {
                None
            },
            if num > 2 {
                Some(unsafe {
                    Handle::from(NonZeroU32::new_unchecked(
                        BufferObject::<T>::handle_for_plane(self, 2).u32_,
                    ))
                })
            } else {
                None
            },
            if num > 3 {
                Some(unsafe {
                    Handle::from(NonZeroU32::new_unchecked(
                        BufferObject::<T>::handle_for_plane(self, 3).u32_,
                    ))
                })
            } else {
                None
            },
        ]
    }
    fn offsets(&self) -> [u32; 4] {
        self.offsets()
    }
}

/// Thrown when the fd is invalid
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidFdError;

impl fmt::Display for InvalidFdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "The returned fd is invalid")
    }
}

impl error::Error for InvalidFdError {}
