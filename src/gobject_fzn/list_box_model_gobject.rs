extern crate libc;
extern crate gobject_sys as gobject_ffi;
extern crate glib_sys as glib_ffi;
extern crate gio_sys as gio_ffi;

use std::ptr;
use std::mem;
use std::collections::HashMap;
use std::any::TypeId;
use std::ffi::CString;
use std::marker::PhantomData;
use self::libc::c_void;
use super::public_interface::ListBoxModel;
use super::GObjectPtrWrapper;
use super::container_gobject::ContainerGObject;

/* Wraps any type implementing public_interface::ListBoxModel
   to be passed into the gtk ffi functions which expect GObjects. */
#[repr(C)]
struct ListBoxModelGObject<T, LBM> where T:'static, LBM: ListBoxModel<T> + 'static {
    parent: gobject_ffi::GObject,
    list_box_model: LBM,
    phantom: PhantomData<T>
}

#[repr(C)]
struct ListBoxModelGObjectClass(gobject_ffi::GObjectClass);

impl<T, LBM> ListBoxModelGObject<T, LBM> where T:'static, LBM: ListBoxModel<T> +'static {

    pub fn new(list_box_model: LBM) -> GObjectPtrWrapper<Self> {
        unsafe {
            let self_gobj_ptr = gobject_ffi::g_object_new(
                Self::get_type(),
                ptr::null()
            ) as *mut Self;

            ptr::write(&mut (*self_gobj_ptr).list_box_model, list_box_model);
            GObjectPtrWrapper::<Self>(self_gobj_ptr)
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

    pub fn get_type() -> glib_ffi::GType {
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

            let type_name_c_string = CString::new(type_name).unwrap();

            let g_type = gobject_ffi::g_type_register_static(
                gobject_ffi::G_TYPE_OBJECT,
                type_name_c_string.as_ptr(),
                &type_info,
                gobject_ffi::GTypeFlags::empty()
            );

            let g_list_model_interface_info = gobject_ffi::GInterfaceInfo {
                interface_init: Some(Self::g_list_model_interface_init),
                interface_finalize: None,
                interface_data: ptr::null_mut()
            };


            // Add GListModel interface.
            gobject_ffi::g_type_add_interface_static(
                g_type,
                gio_ffi::g_list_model_get_type(),
                &g_list_model_interface_info
            );

            types_cache.insert(TypeId::of::<LBM>(), g_type);
            g_type
        }
    }

    extern "C" fn g_list_model_interface_init(g_iface_ptr: glib_ffi::gpointer, _iface_data: glib_ffi::gpointer) {
        let g_list_model_iface_ptr = g_iface_ptr as *mut gio_ffi::GListModelInterface;
        unsafe {
            (*g_list_model_iface_ptr).get_item_type = Some(Self::g_list_model_get_item_type);
            (*g_list_model_iface_ptr).get_n_items = Some(Self::g_list_model_get_n_items);
            (*g_list_model_iface_ptr).get_item = Some(Self::g_list_model_get_item);
        }
    }

    extern "C" fn g_list_model_get_item_type(_list_model_ptr: *mut gio_ffi::GListModel) -> glib_ffi::GType {
        ContainerGObject::<T>::get_type()
    }

    extern "C" fn g_list_model_get_n_items(list_model_ptr: *mut gio_ffi::GListModel) -> u32 {
        let self_ptr = list_model_ptr as *mut Self;
        unsafe {
            (*self_ptr).list_box_model.get_n_items()
        }
    }

    extern "C" fn g_list_model_get_item(list_model_ptr: *mut gio_ffi::GListModel, position: u32) -> glib_ffi::gpointer {
        let self_ptr = list_model_ptr as *mut Self;
        unsafe {
            let g_obj_ptr_wrapper = ContainerGObject::new((*self_ptr).list_box_model.get_item(position));
            g_obj_ptr_wrapper.to_glib_full() as *mut _
        }
    }

}

/* ********************************************************************************************* */


#[cfg(test)]
mod test_object_creation {

    use super::super::test_helpers::RefCountTestDouble;

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
    /* Cloning GObjectPtrWrapper<ContainerGObject> shouldn't cause the contained
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

    extern crate gtk;
    extern crate glib;
    extern crate gtk_sys as gtk_ffi;
    extern crate glib_sys as glib_ffi;
    extern crate gobject_sys as gobject_ffi;
    extern crate gio_sys as gio_ffi;

    use std::ptr;
    use super::super::test_helpers::RefCountTestDouble;
    use super::super::container_gobject::ContainerGObject;
    use super::super::GObjectPtrWrapper;
    use super::super::public_interface::ListBoxModel;
    use super::ListBoxModelGObject;
    use self::gtk::prelude::*;
    use self::glib::translate::*;

    #[derive(Clone)]
    struct StringListItem {
        obj_count: RefCountTestDouble,
        value: String
    }

    struct ListBoxModelTestImpl {
        item: StringListItem
    }

    impl super::ListBoxModel<StringListItem> for ListBoxModelTestImpl {
        fn get_n_items(&self) -> u32 {
            1
        }
        fn get_item(&self, index: u32) -> StringListItem {
            if index == 0 {self.item.clone()}
            else { panic!("Index out of bounds: ListBoxModelTestImpl"); }
        }
    }

    extern "C" fn create_widget(item: *mut gobject_ffi::GObject, _user_data: glib_ffi::gpointer) -> *mut gtk_ffi::GtkWidget {
        println!("Creating widget ");
        unsafe {
            let item = &*(item as *mut ContainerGObject<StringListItem>);
            let label = gtk::Label::new(None);
            label.set_text(&item.get().value.clone());
            label.connect_destroy(move |_| { println!("Destroying Label"); });
            // is this the right way to convert Label to a gpointer?
            label.to_glib_full()
        }
    }

    #[test]
    fn g_is_list_model() {
        let mut ref_count = 0 as isize;
        let list_box_model_gobj = ListBoxModelGObject::<StringListItem, ListBoxModelTestImpl>::new(ListBoxModelTestImpl {
            item: StringListItem {
                obj_count: RefCountTestDouble {ref_count: &mut ref_count as *mut _},
                value: String::from("Hello World!")
            }
        });
        unsafe {
            let instance_ptr = list_box_model_gobj.to_glib_full();
            assert_eq!(gobject_ffi::g_type_check_instance_is_a(instance_ptr as *mut _,
                                                               ListBoxModelGObject::<StringListItem, ListBoxModelTestImpl>::get_type()),
                       1
            );
            assert_eq!(gobject_ffi::g_type_check_instance_is_a(instance_ptr as *mut _,
                                                               gio_ffi::g_list_model_get_type()),
                       1
            );
        }
    }

    /* I'm not sure if this test really belongs here among the unit tests.
       Need to have a look at the new GUI testing library of gtk-rs. */
    #[test]
    fn try_creating_a_list_box() {

        gtk::init();
        let list_box = gtk::ListBox::new();
        let mut ref_count = 0 as isize;
        let list_box_model_gobj = ListBoxModelGObject::new(ListBoxModelTestImpl {
            item: StringListItem {
                obj_count: RefCountTestDouble {ref_count: &mut ref_count as *mut _},
                value: String::from("Hello World!")
            }
        });
        unsafe {
            gtk_ffi::gtk_list_box_bind_model(list_box.to_glib_none().0,
                                             list_box_model_gobj.to_glib_full() as *mut _,
                                             Some(create_widget),
                                             ptr::null_mut(),
                                             None);
        }
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.add(&list_box);
        window.show_all();
        gtk::main();

    }
}