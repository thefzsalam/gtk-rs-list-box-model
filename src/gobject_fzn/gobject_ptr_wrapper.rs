extern crate gobject_sys as gobject_ffi;

use std::ops::Deref;

/* A wrapper for pointers returned by
   `g_object_new`,
   mainly to call g_object_ref and g_object_unref
   in RAII style.
*/
pub struct GObjectPtrWrapper<T>(pub *mut T);

impl<T> GObjectPtrWrapper<T> {
    /* Stop taking care of the GObject associated
       with pointer self.0.
       After calling this method,
       the caller must assume the responsibility
       of freeing the resource afterwards.
    */
    pub fn to_glib_full(self) -> *mut T {
        self.0
    }
}

impl<T> Clone for GObjectPtrWrapper<T> {
    fn clone(&self) -> Self {
        unsafe {gobject_ffi::g_object_ref(self.0 as *mut gobject_ffi::GObject);}
        GObjectPtrWrapper::<T>(self.0)
    }
}

impl<T> Drop for GObjectPtrWrapper<T> {
    fn drop(&mut self) {
        unsafe {gobject_ffi::g_object_unref(self.0 as *mut gobject_ffi::GObject);}
    }
}

impl<T> Deref for GObjectPtrWrapper<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe{&*self.0}
    }
}