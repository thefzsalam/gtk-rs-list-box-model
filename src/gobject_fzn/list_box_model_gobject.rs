extern crate libc;
extern crate gobject_sys as gobject_ffi;
extern crate glib_sys as glib_ffi;

use std::ptr;
use std::mem;
use std::collections::HashMap;
use std::any::TypeId;
use std::ffi::CString;
use std::marker::PhantomData;
use self::libc::c_void;
use super::public_interface::ListBoxModel;
use super::GObjectWrapper;


/* Wraps any type implementing public_interface::ListBoxModel
   to be passed into the gtk ffi functions which expect GObjects. */
#[repr(C)]
struct ListBoxModelGObject<T, LBM> where LBM: ListBoxModel<T> + 'static {
    parent: gobject_ffi::GObject,
    list_box_model: LBM,
    phantom: PhantomData<T>
}

#[repr(C)]
struct ListBoxModelGObjectClass(gobject_ffi::GObjectClass);

impl<T, LBM> ListBoxModelGObject<T, LBM> where LBM: ListBoxModel<T> +'static {

    pub fn new(list_box_model: LBM) -> GObjectWrapper<Self> {
        unsafe {
            let self_gobj_ptr = gobject_ffi::g_object_new(
                Self::get_type(),
                ptr::null()
            ) as *mut Self;

            ptr::write(&mut (*self_gobj_ptr).list_box_model, list_box_model);
            GObjectWrapper::<Self>(self_gobj_ptr)
        }
    }

    extern "C" fn class_init(klass_ptr: *mut c_void, _data: *mut c_void) {
        let klass_ptr = klass_ptr as *mut gobject_ffi::GObjectClass;
        unsafe { (*klass_ptr).dispose = Some(Self::dispose); }
    }

    extern "C" fn dispose(gobj_ptr: *mut gobject_ffi::GObject) {
        unsafe {
            let self_ptr = gobj_ptr as *mut Self;
            ptr::drop_in_place(&mut (*self_ptr).list_box_model);
            let parent_class: *mut gobject_ffi::GObjectClass = gobject_ffi::g_type_class_peek_parent(
                gobject_ffi::g_type_class_peek_static(Self::get_type())
            ) as *mut _;
            let parent_dispose_fn = (*parent_class).dispose.unwrap();
            parent_dispose_fn(gobj_ptr);
        }
    }

    fn get_type() -> glib_ffi::GType {
        unsafe {
            static mut TYPES_CACHE: Option<HashMap<TypeId, glib_ffi::GType>> = None;
            static mut TYPE_INDEX: usize = 0;

            if TYPES_CACHE == None {
                TYPES_CACHE = Some(HashMap::new());
            }
            let types_cache = TYPES_CACHE.as_mut().unwrap();

            /* Note that TYPES_CACHE caches GType based on LBM,
               and not T.
               If one defines SimpleListBoxModel<T> and ExtraCoolListBoxModel<T>,
               using TypeId::of::<T>() to cache the GType will cause conflicts. */

            if let Some(g_type) = types_cache.get(&TypeId::of::<LBM>()) {
                return *g_type;
            }

            let type_info = gobject_ffi::GTypeInfo {
                class_size: mem::size_of::<ListBoxModelGObjectClass>() as u16,
                base_init: None,
                base_finalize: None,
                class_init: Some(Self::class_init),
                class_finalize: None,
                class_data: ptr::null(),
                instance_size: mem::size_of::<ListBoxModelGObject<T, LBM>>() as u16,
                n_preallocs: 0,
                instance_init: None,
                value_table: ptr::null()
            };

            // to ensure a unique type_name
            let type_name = String::from("RustListBoxModelGObject")+&TYPE_INDEX.to_string();
            TYPE_INDEX += 1;

            let g_type = gobject_ffi::g_type_register_static(
                gobject_ffi::G_TYPE_OBJECT,
                CString::new(type_name).unwrap().as_ptr(),
                &type_info as *const _,
                gobject_ffi::GTypeFlags::empty()
            );
            types_cache.insert(TypeId::of::<LBM>(), g_type);
            g_type
        }
    }
}

/* ********************************************************************************************* */


#[cfg(test)]
mod test_object_creation {
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

    impl super::ListBoxModel<u8> for RefCountTestDouble {
        fn get_n_items(&self) -> u32 {unimplemented!()}
        fn get_item(&self, _index: u32) -> u8 {unimplemented!()}
    }

    #[test]
    /* Did object construction succeed?
       If mistakes are made in GObject registration code,
       SIGSEGV awaits you. */
    fn construction_works() {
        struct U8LBM();
        impl super::ListBoxModel<u8> for U8LBM {
            fn get_n_items(&self) -> u32 {0}
            fn get_item(&self, _index: u32) -> u8 {unimplemented!()}
        }
        let _lbm_gobj = super::ListBoxModelGObject::new(U8LBM());
    }

    #[test]
    /* Make sure that get_type::<T>() registers different types for different values of T. */
    fn multiple_type_construction_works() {
        struct StringLBM();
        struct U8LBM();
        impl super::ListBoxModel<String> for StringLBM {
            fn get_n_items(&self) -> u32 {unimplemented!()}
            fn get_item(&self, _index: u32) -> String {unimplemented!()}
        }
        impl super::ListBoxModel<u8> for U8LBM {
            fn get_n_items(&self) -> u32 {unimplemented!()}
            fn get_item(&self, _index: u32) -> u8 {unimplemented!()}
        }
        let _lbm_string_gobj = super::ListBoxModelGObject::new(StringLBM());
        let _lbm_u8_gobj = super::ListBoxModelGObject::new(U8LBM());
    }

    #[test]
    /* Test, with the help of RefcountTestDouble,
       gobject refcount is decremented, disposed is called when refcount == 0,
       and the contained values are dropped safely in dispose() method.*/
    fn destructed_properly() {
        let ref_count: *mut isize = Box::into_raw(Box::<isize>::new(1));
        let ref_count_dummy = RefCountTestDouble{ref_count};
        {
            unsafe{assert_eq!(*ref_count,1);}
            let _ptr_object = super::ListBoxModelGObject::new(ref_count_dummy);
            unsafe{assert_eq!(*ref_count,1);}
        }
        unsafe{assert_eq!(*ref_count,0);}
    }

    #[test]
    /* Cloning GObjectWrapper<ContainerGObject> shouldn't cause the contained
       type to be cloned. */
    fn clone_destructed_properly() {
        let ref_count: *mut isize = Box::into_raw(Box::<isize>::new(1));
        let ref_count_dummy = RefCountTestDouble{ref_count};
        {
            unsafe{assert_eq!(*ref_count,1);}
            let _ptr_object1 = super::ListBoxModelGObject::new(ref_count_dummy);
            let _ptr_object2 = _ptr_object1.clone();
            let _ptr_object3 = _ptr_object2.clone();
            let _ptr_object4 = _ptr_object3.clone();
            unsafe{assert_eq!(*ref_count,1);}
        }
        unsafe{assert_eq!(*ref_count,0);}
    }
}

#[cfg(test)]
mod test_list_box_functionality {

}