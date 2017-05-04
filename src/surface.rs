use std::marker::PhantomData;
use std::mem;

use ::{AsRaw, FromRaw};
use ::BufferObject;

pub struct Surface<'a> {
    ffi: *mut ::ffi::gbm_surface,
    _lifetime: PhantomData<&'a ()>,
}

pub enum FrontBufferError {
    NoFreeBuffers,
    Unknown,
}

impl<'a> Surface<'a> {
    pub fn needs_lock_front_buffer(&self) -> bool {
        unsafe { ::ffi::gbm_surface_needs_lock_front_buffer(self.ffi) != 0 }
    }

    pub fn has_free_buffers(&self) -> bool {
        unsafe { ::ffi::gbm_surface_has_free_buffers(self.ffi) != 0 }
    }

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
