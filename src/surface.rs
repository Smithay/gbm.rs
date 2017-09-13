use {AsRaw, FromRaw};
use BufferObject;
use std::error::{self, Error};
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};

/// A gbm rendering surface
pub struct Surface<'a, T: 'static> {
    ffi: *mut ::ffi::gbm_surface,
    _lifetime: PhantomData<&'a ()>,
    _bo_userdata: PhantomData<T>,
}

/// Handle to a front buffer of a surface
pub struct SurfaceBufferHandle<'a, T: 'static>(&'a Surface<'a, T>, Option<BufferObject<'a, T>>);

impl<'a, T: 'static> Deref for SurfaceBufferHandle<'a, T> {
    type Target = BufferObject<'a, T>;

    fn deref(&self) -> &Self::Target {
        self.1.as_ref().unwrap()
    }
}

impl<'a, T: 'static> DerefMut for SurfaceBufferHandle<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.as_mut().unwrap()
    }
}

impl<'a, T: 'static> Drop for SurfaceBufferHandle<'a, T> {
    fn drop(&mut self) {
        let mut bo = None;
        mem::swap(&mut bo, &mut self.1);
        unsafe { ::ffi::gbm_surface_release_buffer(self.0.ffi, bo.as_mut().unwrap().as_raw_mut()) };
        mem::forget(bo); // don't drop
    }
}

/// Errors that may happen when locking the front buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontBufferError {
    /// No free buffers are currently available
    NoFreeBuffers,
    /// An unknown error happened
    Unknown,
}

impl fmt::Display for FrontBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl error::Error for FrontBufferError {
    fn description(&self) -> &str {
        match self {
            &FrontBufferError::NoFreeBuffers => "No free buffers remaining",
            &FrontBufferError::Unknown => "Unknown error",
        }
    }
    fn cause(&self) -> Option<&error::Error> { None }
}

impl<'a, T: 'static> Surface<'a, T> {
    ///  Return whether or not a surface has free (non-locked) buffers
    ///
    /// Before starting a new frame, the surface must have a buffer
    /// available for rendering.  Initially, a gbm surface will have a free
    /// buffer, but after one or more buffers
    /// [have been locked](#method.lock_front_buffer),
    /// the application must check for a free buffer before rendering.
    pub fn has_free_buffers(&self) -> bool {
        unsafe { ::ffi::gbm_surface_has_free_buffers(self.ffi) != 0 }
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
    pub fn lock_front_buffer(&'a self) -> Result<SurfaceBufferHandle<'a, T>, FrontBufferError> {
        if unsafe { ::ffi::gbm_surface_has_free_buffers(self.ffi) != 0 } {
            let buffer_ptr = unsafe { ::ffi::gbm_surface_lock_front_buffer(self.ffi) };
            if !buffer_ptr.is_null() {
                let buffer = unsafe { BufferObject::from_raw(buffer_ptr) };
                Ok(SurfaceBufferHandle(self, Some(buffer)))
            } else {
                Err(FrontBufferError::Unknown)
            }
        } else {
            Err(FrontBufferError::NoFreeBuffers)
        }
    }
}

impl<'a, T: 'static> AsRaw<::ffi::gbm_surface> for Surface<'a, T> {
    fn as_raw(&self) -> *const ::ffi::gbm_surface {
        self.ffi
    }
}

impl<'a, T: 'static> FromRaw<::ffi::gbm_surface> for Surface<'a, T> {
    unsafe fn from_raw(ffi: *mut ::ffi::gbm_surface) -> Self {
        Surface {
            ffi: ffi,
            _lifetime: PhantomData,
            _bo_userdata: PhantomData,
        }
    }
}

impl<'a, T: 'static> Drop for Surface<'a, T> {
    fn drop(&mut self) {
        while self.has_free_buffers() {
            if let Ok(mut buffer) = self.lock_front_buffer() {
                buffer.take_userdata();
            } else {
                break;
            }
        }
        unsafe { ::ffi::gbm_surface_destroy(self.ffi) }
    }
}
