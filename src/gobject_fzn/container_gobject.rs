extern crate libc;
extern crate gobject_sys as gobject_ffi;
extern crate glib_sys as glib_ffi;
extern crate gtk_sys as gtk_ffi;

use std::ptr;
use std::mem;
use std::ffi::CString;
use std::collections::hash_map::HashMap;
use std::any::TypeId;
use self::libc::c_void;
use super::GObjectWrapper;

/* To make any rust type returnable
   from Gtk/GObject related interfaces,
   like GListModel for GtkListBox.

   This is a structure which follows GObject rules,
   registered in GObject type system,
   takes ownership of a value during construction
   and stores it inside the structure.

   Instances are allocated by GObject on the heap,
   thus the embedded rust instance `value: T`
   lives in the heap.

   The constructor further wraps this in GObjectWrapper
   to facilitate ref counting. */
#[repr(C)]
pub struct ContainerGObject<T>
where T: 'static {
    parent: gobject_ffi::GObject,
    value: T
}

/* GObject needs this struct. Read up the GObject docs to find out why :) */
#[repr(C)]
struct ContainerGObjectClass(gobject_ffi::GObjectClass);

impl<T> ContainerGObject<T>
where T: 'static {
    /* Will take ownership of value.
       When ref count of this GObject reaches zero,
       the GObject type system will call the
       `dispose()` method on this object, which will drop `value`.
    */
    pub fn new(value: T) -> GObjectWrapper<Self> {
        unsafe {
            let gobj_ptr = gobject_ffi::g_object_new(
                Self::get_type(),
                ptr::null()
            );
            let self_ptr = gobj_ptr as *mut Self;
            ptr::write(&mut (*self_ptr).value, value);
            GObjectWrapper::<Self>(self_ptr)
        }
    }

    extern "C" fn class_init(klass_ptr: *mut c_void, _data: *mut c_void) {
        let klass_ptr = klass_ptr as *mut gobject_ffi::GObjectClass;
        unsafe { (*klass_ptr).dispose = Some(Self::dispose); }
    }

    extern "C" fn dispose(gobj_ptr: *mut gobject_ffi::GObject) {
        unsafe {
            let self_ptr = gobj_ptr as *mut Self;
            ptr::drop_in_place(&mut (*self_ptr).value);
            let parent_class: *mut gobject_ffi::GObjectClass = gobject_ffi::g_type_class_peek_parent(
                gobject_ffi::g_type_class_peek_static(Self::get_type())
            ) as *mut _;
            let parent_dispose_fn = (*parent_class).dispose.unwrap();
            parent_dispose_fn(gobj_ptr);
        }
    }

    extern "C" fn get_type() -> glib_ffi::GType {
        unsafe {
            static mut TYPES_CACHE: Option<HashMap<TypeId, glib_ffi::GType>> = None;
            static mut TYPE_INDEX: usize = 0;

            if TYPES_CACHE == None {
                TYPES_CACHE = Some(HashMap::new());
            }
            let types_cache = TYPES_CACHE.as_mut().unwrap();


            if let Some(g_type) = types_cache.get(&TypeId::of::<T>()) {
                return *g_type;
            }

            let type_info = gobject_ffi::GTypeInfo {
                class_size: mem::size_of::<ContainerGObjectClass>() as u16,
                base_init: None,
                base_finalize: None,
                class_init: Some(Self::class_init),
                class_finalize: None,
                class_data: ptr::null(),
                instance_size: mem::size_of::<ContainerGObject<T>>() as u16,
                n_preallocs: 0,
                instance_init: None,
                value_table: ptr::null()
            };

            // to ensure a unique type_name
            let type_name = String::from("RustContainerGObject")+&TYPE_INDEX.to_string();
            TYPE_INDEX += 1;

            let g_type = gobject_ffi::g_type_register_static(
                gobject_ffi::G_TYPE_OBJECT,
                CString::new(type_name).unwrap().as_ptr(),
                &type_info as *const _,
                gobject_ffi::GTypeFlags::empty()
            );
            types_cache.insert(TypeId::of::<T>(), g_type);
            g_type
        }
    }

}

#[cfg(test)]
mod test {
    /* Accepts a pointer to an existing variable.
       Will increment/decrement it when an instance is
       cloned/dropped respectively.
       Used to test for clones and drops. */
    struct RefCountTestDouble{
        ref_count: *mut isize
    }

    impl Drop for RefCountTestDouble {
        fn drop(&mut self) {
            unsafe {(*self.ref_count) -= 1;}
        }
    }

    impl Clone for RefCountTestDouble {
        fn clone(&self) -> Self {
            unsafe {(*self.ref_count) += 1;}
            Self {
                ref_count: self.ref_count
            }
        }
    }

    #[test]
    /* Did object construction succeed?
       If mistakes are made in GObject registration code,
       SIGSEGV awaits you. */
    fn construction_works() {
        let _container_gobj = super::ContainerGObject::new(String::from("Hello World!"));
    }

    #[test]
    /* Make sure that get_type::<T>() registers different types for different values of T. */
    fn multiple_type_construction_works() {
        let _container_gobj_string = super::ContainerGObject::new(String::from("Hello World!"));
        let _container_gobj_u64 = super::ContainerGObject::new(123 as u64);
        let _container_gobj_vec_u8 = super::ContainerGObject::new(vec![1,2,3]);
    }

    #[test]
    /* Test, with the help of RefcountTestDouble that,
       gobject refcounting works as expected and
       drop method is called on the contained value
       when gobject refcount reaches zero. */
    fn destructed_properly() {
        let ref_count: *mut isize = Box::into_raw(Box::<isize>::new(1));
        let ref_count_dummy = RefCountTestDouble{ref_count};
        {
            unsafe{assert_eq!(*ref_count,1);}
            let _ptr_object = super::ContainerGObject::new(ref_count_dummy);
            unsafe{assert_eq!(*ref_count,1);}
        }
        unsafe{assert_eq!(*ref_count,0);}
    }

    #[test]
    /* GObject refcounting and dispose() works after cloning. */
    fn clone_destructed_properly() {
        let ref_count: *mut isize = Box::into_raw(Box::<isize>::new(1));
        let ref_count_dummy = RefCountTestDouble{ref_count};
        {
            unsafe{assert_eq!(*ref_count,1);}
            let _ptr_object1 = super::ContainerGObject::new(ref_count_dummy);
            let _ptr_object2 = _ptr_object1.clone();
            let _ptr_object3 = _ptr_object2.clone();
            let _ptr_object4 = _ptr_object3.clone();
            unsafe{assert_eq!(*ref_count,1);}
        }
        unsafe{assert_eq!(*ref_count,0);}
    }
}