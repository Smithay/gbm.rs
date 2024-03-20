use crate::{AsRaw, BufferObject, DeviceDestroyedError, Ptr, WeakPtr};
use std::error;
use std::fmt;
use std::marker::PhantomData;

/// A GBM rendering surface
pub struct Surface<T: 'static> {
    ffi: Ptr<ffi::gbm_surface>,
    _device: WeakPtr<ffi::gbm_device>,
    _bo_userdata: PhantomData<T>,
}

impl<T: 'static> fmt::Debug for Surface<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Surface")
            .field("ptr", &format_args!("{:p}", &self.ffi))
            .field("device", &format_args!("{:p}", &self._device))
            .finish()
    }
}

/// Errors that may happen when locking the front buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontBufferError {
    /// An unknown error happened
    Unknown,
    /// Device was already released
    Destroyed(DeviceDestroyedError),
}

impl fmt::Display for FrontBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FrontBufferError::Unknown => write!(f, "Unknown error"),
            FrontBufferError::Destroyed(ref err) => write!(f, "Buffer was destroyed: {}", err),
        }
    }
}

impl error::Error for FrontBufferError {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            FrontBufferError::Destroyed(ref err) => Some(err),
            _ => None,
        }
    }
}

impl<T: 'static> Surface<T> {
    ///  Return whether or not a surface has free (non-locked) buffers
    ///
    /// Before starting a new frame, the surface must have a buffer
    /// available for rendering.  Initially, a GBM surface will have a free
    /// buffer, but after one or more buffers
    /// [have been locked](Self::lock_front_buffer()),
    /// the application must check for a free buffer before rendering.
    pub fn has_free_buffers(&self) -> bool {
        let device = self._device.upgrade();
        if device.is_some() {
            unsafe { ffi::gbm_surface_has_free_buffers(*self.ffi) != 0 }
        } else {
            false
        }
    }

    /// Lock the surface's current front buffer
    ///
    /// Locks rendering to the surface's current front buffer and returns
    /// a handle to the underlying [`BufferObject`].
    ///
    /// If an error occurs a [`FrontBufferError`] is returned.
    ///
    /// # Safety
    /// This function must be called exactly once after calling
    /// `eglSwapBuffers`.  Calling it before any `eglSwapBuffers` has happened
    /// on the surface or two or more times after `eglSwapBuffers` is an
    /// error and may cause undefined behavior.
    pub unsafe fn lock_front_buffer(&self) -> Result<BufferObject<T>, FrontBufferError> {
        let device = self._device.upgrade();
        if device.is_some() {
            let buffer_ptr = ffi::gbm_surface_lock_front_buffer(*self.ffi);
            if !buffer_ptr.is_null() {
                let surface_ptr = self.ffi.downgrade();
                let buffer = BufferObject {
                    ffi: Ptr::new(buffer_ptr, move |ptr| {
                        if let Some(surface) = surface_ptr.upgrade() {
                            ffi::gbm_surface_release_buffer(*surface, ptr);
                        }
                    }),
                    _device: self._device.clone(),
                    _userdata: std::marker::PhantomData,
                };
                Ok(buffer)
            } else {
                Err(FrontBufferError::Unknown)
            }
        } else {
            Err(FrontBufferError::Destroyed(DeviceDestroyedError))
        }
    }

    pub(crate) unsafe fn new(
        ffi: *mut ffi::gbm_surface,
        device: WeakPtr<ffi::gbm_device>,
    ) -> Surface<T> {
        Surface {
            ffi: Ptr::new(ffi, |ptr| ffi::gbm_surface_destroy(ptr)),
            _device: device,
            _bo_userdata: PhantomData,
        }
    }
}

impl<T: 'static> AsRaw<ffi::gbm_surface> for Surface<T> {
    fn as_raw(&self) -> *const ffi::gbm_surface {
        *self.ffi
    }
}
