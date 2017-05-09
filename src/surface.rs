use std::marker::PhantomData;
use std::mem;

use ::{AsRaw, FromRaw};
use ::BufferObject;

/// A gbm rendering surface
pub struct Surface<'a> {
    ffi: *mut ::ffi::gbm_surface,
    _lifetime: PhantomData<&'a ()>,
}

/// Errors that may happen when locking the front buffer
pub enum FrontBufferError {
    /// No free buffers are currently available
    NoFreeBuffers,
    /// An unknown error happened
    Unknown,
}

impl<'a> Surface<'a> {
    ///  Return whether or not a surface has free (non-locked) buffers
    ///
    /// Before starting a new frame, the surface must have a buffer
    /// available for rendering.  Initially, a gbm surface will have a free
    /// buffer, but after one or more buffers
    /// [have been locked](#method.with_locked_front_buffer),
    /// the application must check for a free buffer before rendering.
    pub fn has_free_buffers(&self) -> bool {
        unsafe { ::ffi::gbm_surface_has_free_buffers(self.ffi) != 0 }
    }

    /// Lock the surface's current front buffer
    ///
    /// Locks rendering to the surface's current front buffer for a given closure
    /// and releases the lock after the closure has returned.
    ///
    /// This function must be called exactly once after calling
    /// `eglSwapBuffers`.  Calling it before any `eglSwapBuffer` has happened
    /// on the surface or two or more times after `eglSwapBuffers` is an
    /// error.
    ///
    /// If an error occurs a `FrontBufferError` is returned.
    pub fn with_locked_front_buffer<F>(&mut self, func: F) -> Result<(), FrontBufferError>
        where F: FnOnce(&mut BufferObject<()>)
    {
        if unsafe { ::ffi::gbm_surface_has_free_buffers(self.ffi) != 0 } {
            let buffer_ptr = unsafe { ::ffi::gbm_surface_lock_front_buffer(self.ffi) };
            if !buffer_ptr.is_null() {
                let mut buffer = unsafe {
                    BufferObject::from_raw(buffer_ptr)
                };
                func(&mut buffer);
                unsafe {
                    ::ffi::gbm_surface_release_buffer(self.ffi, buffer.as_raw_mut());
                    mem::forget(buffer);
                }
                Ok(())
            } else {
                Err(FrontBufferError::Unknown)
            }
        } else {
            Err(FrontBufferError::NoFreeBuffers)
        }
    }
}

impl<'a> AsRaw<::ffi::gbm_surface> for Surface<'a> {
    fn as_raw(&self) -> *const ::ffi::gbm_surface {
        self.ffi
    }
}

impl<'a> FromRaw<::ffi::gbm_surface> for Surface<'a> {
    unsafe fn from_raw(ffi: *mut ::ffi::gbm_surface) -> Self {
        Surface {
            ffi: ffi,
            _lifetime: PhantomData,
        }
    }
}

impl<'a> Drop for Surface<'a> {
    fn drop(&mut self) {
        unsafe { ::ffi::gbm_surface_destroy(self.ffi) }
    }
}
