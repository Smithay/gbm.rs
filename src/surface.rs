use {AsRaw, BufferObject, DeviceDestroyedError};
use std::error::{self, Error};
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};

/// A gbm rendering surface
pub struct Surface<T: 'static> {
    ffi: Rc<*mut ::ffi::gbm_surface>,
    _device: Weak<*mut ::ffi::gbm_device>,
    _bo_userdata: PhantomData<T>,
}

/// Handle to a front buffer of a surface
pub struct SurfaceBufferHandle<T: 'static>(Weak<*mut ::ffi::gbm_surface>, Option<BufferObject<T>>);

impl<T: 'static> Deref for SurfaceBufferHandle<T> {
    type Target = BufferObject<T>;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref().unwrap()
    }
}

impl<T: 'static> DerefMut for SurfaceBufferHandle<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.as_mut().unwrap()
    }
}

impl<T: 'static> Drop for SurfaceBufferHandle<T> {
    fn drop(&mut self) {
        if let Some(surface_ptr) = self.0.upgrade() {
            if self.1.as_ref().unwrap()._device.upgrade().is_some() {
                let mut bo = None;
                mem::swap(&mut bo, &mut self.1);
                unsafe { ::ffi::gbm_surface_release_buffer(*surface_ptr, bo.as_mut().unwrap().as_raw_mut()) };
                mem::forget(bo); // don't drop
            }
        }
    }
}

/// Errors that may happen when locking the front buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontBufferError {
    /// No free buffers are currently available
    NoFreeBuffers,
    /// An unknown error happened
    Unknown,
    /// Device was already released
    Destroyed(DeviceDestroyedError),
}

impl fmt::Display for FrontBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl error::Error for FrontBufferError {
    fn description(&self) -> &str {
        match *self {
            FrontBufferError::NoFreeBuffers => "No free buffers remaining",
            FrontBufferError::Unknown => "Unknown error",
            FrontBufferError::Destroyed(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
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
    /// available for rendering.  Initially, a gbm surface will have a free
    /// buffer, but after one or more buffers
    /// [have been locked](#method.lock_front_buffer),
    /// the application must check for a free buffer before rendering.
    pub fn has_free_buffers(&self) -> bool {
        if self._device.upgrade().is_some() {
            unsafe { ::ffi::gbm_surface_has_free_buffers(*self.ffi) != 0 }
        } else {
            false
        }
    }

    /// Lock the surface's current front buffer
    ///
    /// Locks rendering to the surface's current front buffer and returns
    /// a handle to the underlying `BufferObject`
    ///
    /// This function must be called exactly once after calling
    /// `eglSwapBuffers`.  Calling it before any `eglSwapBuffer` has happened
    /// on the surface or two or more times after `eglSwapBuffers` is an
    /// error.
    ///
    /// If an error occurs a `FrontBufferError` is returned.
    pub fn lock_front_buffer(&self) -> Result<SurfaceBufferHandle<T>, FrontBufferError> {
        if self._device.upgrade().is_some() {
            if unsafe { ::ffi::gbm_surface_has_free_buffers(*self.ffi) != 0 } {
                let buffer_ptr = unsafe { ::ffi::gbm_surface_lock_front_buffer(*self.ffi) };
                if !buffer_ptr.is_null() {
                    let buffer = unsafe { BufferObject::new(buffer_ptr, self._device.clone()) };
                    Ok(SurfaceBufferHandle(Rc::downgrade(&self.ffi), Some(buffer)))
                } else {
                    Err(FrontBufferError::Unknown)
                }
            } else {
                Err(FrontBufferError::NoFreeBuffers)
            }
        } else {
            Err(FrontBufferError::Destroyed(DeviceDestroyedError))
        }
    }

    pub(crate) unsafe fn new(ffi: *mut ::ffi::gbm_surface, device: Weak<*mut ::ffi::gbm_device>) -> Surface<T> {
        Surface {
            ffi: Rc::new(ffi),
            _device: device,
            _bo_userdata: PhantomData,
        }
    }
}

impl<T: 'static> AsRaw<::ffi::gbm_surface> for Surface<T> {
    fn as_raw(&self) -> *const ::ffi::gbm_surface {
        *self.ffi
    }
}

impl<T: 'static> Drop for Surface<T> {
    fn drop(&mut self) {
        if self._device.upgrade().is_some() {
            while self.has_free_buffers() {
                if let Ok(mut buffer) = self.lock_front_buffer() {
                    buffer.take_userdata().unwrap();
                } else {
                    break;
                }
            }
            unsafe { ::ffi::gbm_surface_destroy(*self.ffi) }
        }
    }
}
