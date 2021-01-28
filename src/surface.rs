use std::error;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};
use {AsRaw, BufferObject, DeviceDestroyedError};

/// A gbm rendering surface
pub struct Surface<T: 'static> {
    ffi: Rc<*mut ::ffi::gbm_surface>,
    _device: Weak<*mut ::ffi::gbm_device>,
    _bo_userdata: PhantomData<T>,
}

#[cfg(feature = "glutin-support")]
use glutin_interface::{NativeWindow, RawWindow, Seal};

#[cfg(feature = "glutin-support")]
use winit_types::dpi::PhysicalSize;

#[cfg(feature = "glutin-support")]
impl<T: 'static> NativeWindow for Surface<T> {
    fn raw_window(&self) -> RawWindow {
        RawWindow::Gbm {
            gbm_surface: *self.ffi as *mut _,
            _non_exhaustive_do_not_use: Seal,
        }
    }

    fn size(&self) -> PhysicalSize<u32> {
        // Glutin doesn't need this for this platform, so whatever
        unimplemented!()
    }

    fn scale_factor(&self) -> f64 {
        1.0
    }
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
        let err = match *self {
            FrontBufferError::NoFreeBuffers => "No free buffers remaining".to_string(),
            FrontBufferError::Unknown => "Unknown error".to_string(),
            FrontBufferError::Destroyed(ref err) => err.to_string(),
        };
        write!(f, "{}", err)
    }
}

impl error::Error for FrontBufferError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
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
    /// If an error occurs a `FrontBufferError` is returned.
    ///
    /// **Unsafety**: This function must be called exactly once after calling
    /// `eglSwapBuffers`.  Calling it before any `eglSwapBuffer` has happened
    /// on the surface or two or more times after `eglSwapBuffers` is an
    /// error and may cause undefined behavior.
    pub unsafe fn lock_front_buffer(&self) -> Result<SurfaceBufferHandle<T>, FrontBufferError> {
        if self._device.upgrade().is_some() {
            if ::ffi::gbm_surface_has_free_buffers(*self.ffi) != 0 {
                let buffer_ptr = ::ffi::gbm_surface_lock_front_buffer(*self.ffi);
                if !buffer_ptr.is_null() {
                    let buffer = BufferObject::new(buffer_ptr, self._device.clone());
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

    pub(crate) unsafe fn new(
        ffi: *mut ::ffi::gbm_surface,
        device: Weak<*mut ::ffi::gbm_device>,
    ) -> Surface<T> {
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
            unsafe { ::ffi::gbm_surface_destroy(*self.ffi) }
        }
    }
}
