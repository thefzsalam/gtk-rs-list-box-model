extern crate gobject_sys as gobject_ffi;

use std::ops::Deref;

/* A wrapper for pointers returned by
   `g_object_new`,
   mainly to call g_object_ref and g_object_unref
   in RAII style.
*/
pub struct GObjectWrapper<T>(pub *mut T);

impl<T> GObjectWrapper<T> {
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

impl<T> Clone for GObjectWrapper<T> {
    fn clone(&self) -> Self {
        unsafe {gobject_ffi::g_object_ref(self.0 as *mut gobject_ffi::GObject);}
        GObjectWrapper::<T>(self.0)
    }
}

impl<T> Drop for GObjectWrapper<T> {
    fn drop(&mut self) {
        unsafe {gobject_ffi::g_object_unref(self.0 as *mut gobject_ffi::GObject);}
    }
}

impl<T> Deref for GObjectWrapper<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe{&*self.0}
    }
}