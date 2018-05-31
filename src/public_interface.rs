extern crate gtk;
extern crate gtk_sys as gtk_ffi;
extern crate gobject_sys as gobject_ffi;
extern crate glib_sys as glib_ffi;
extern crate glib;

use self::glib::translate::*;
use std::mem;
use super::list_box_model_gobject::ListBoxModelGObject;
use super::container_gobject::ContainerGObject;


pub trait ListBoxModel<T> {
    fn get_n_items(&self) -> u32;
    fn get_item(&self, index: u32) -> T;
}

/* Shall I turn this into a trait and implement it on gtk::ListBox ? */
pub fn gtk_list_box_bind_model<T, LBM, WC>(list_box: &gtk::ListBox,
                                       list_box_model: LBM,
                                       widget_creator: WC )
where T:'static, LBM: ListBoxModel<T>+'static, WC: Fn(&T)->gtk::Widget   {

    extern "C" fn create_widget<T, WC> (item: *mut gobject_ffi::GObject, user_data: glib_ffi::gpointer)
                                        -> *mut gtk_ffi::GtkWidget
    where T:'static, WC: Fn(&T)->gtk::Widget {
        let container_gobj: *mut ContainerGObject<T> = item as *mut _;
        let widget_creator = user_data as *mut WC;
        unsafe { (*widget_creator)((*container_gobj).get()).to_glib_full() }
    }

    extern "C" fn user_data_free_func<WC>(user_data: glib_ffi::gpointer) {
        let widget_creator = user_data as *mut WC;
        unsafe { mem::drop(Box::from_raw(widget_creator));}
    }
    unsafe {
        gtk_ffi::gtk_list_box_bind_model(
            list_box.to_glib_none().0,
            ListBoxModelGObject::new(list_box_model).to_glib_full() as *mut _,
            Some(create_widget::<T, WC>),
            Box::into_raw(Box::from(widget_creator))  as glib_ffi::gpointer,
            Some(user_data_free_func::<WC>)
        );
    }
}

#[cfg(test)]
mod test {

    extern crate gtk;
    extern crate glib;

    use self::gtk::prelude::*;

    struct MyList<T> {
        items: Vec<T>
    }
    impl<T: Clone> super::ListBoxModel<T> for MyList<T> {
        fn get_n_items(&self) -> u32 {
            self.items.len() as _
        }

        fn get_item(&self, index: u32) -> T {
            self.items[index as usize].clone()
        }
    }

    fn create_widget_fn(s: &String) -> gtk::Widget {
        let label = gtk::Label::new(None);
        label.set_text(s);
        label.upcast()
    }
    #[test]
    fn try_creating_a_list_box() {
        gtk::init();
        let list_box = gtk::ListBox::new();
        let my_list = MyList::<String> {
            items: vec![
                String::from("Kingdom"),
                String::from("Phylum"),
                String::from("Class"),
                String::from("Order"),
                String::from("Family"),
                String::from("Genus"),
                String::from("Species")
            ]
        };

        super::gtk_list_box_bind_model(&list_box, my_list, create_widget_fn);

        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.add(&list_box);
        window.show_all();
        window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });
        gtk::main();
    }
}