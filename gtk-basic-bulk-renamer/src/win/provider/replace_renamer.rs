use crate::win::provider::ProviderCommon;
use gtk::prelude::*;
use gtk::{Builder, CheckButton, Container, Entry};
use std::cell::RefCell;
use std::rc::Rc;
use std::vec::IntoIter;

const ID_REPLACE_RENAMER_PANEL: &'static str = "replace-renamer-panel";
const ID_PATTERN_ENTRY: &'static str = "pattern-entry";
const ID_REGEXP_SUPPORTED: &'static str = "regexp-supported";
const ID_REPLACEMENT_ENTRY: &'static str = "replacement-entry";
const ID_CASE_SENSITIVE: &'static str = "case-sensitive";

pub struct ReplaceRenamer {
    builder: Builder,
    callback: Option<Rc<RefCell<dyn Fn()>>>,
}

impl ReplaceRenamer {
    pub fn new(callback: Option<Rc<RefCell<dyn Fn()>>>) -> Self {
        let builder = Builder::from_string(include_str!("replace_renamer.glade"));
        let renamer = Self { builder, callback };

        renamer.init_callback();

        renamer
    }

    fn init_callback(&self) {
        if let Some(callback) = &self.callback {
            let pattern_entry = self.get_object::<Entry>(ID_PATTERN_ENTRY);
            let regexp_supported = self.get_object::<CheckButton>(ID_REGEXP_SUPPORTED);
            let replacement_entry = self.get_object::<Entry>(ID_REPLACEMENT_ENTRY);
            let case_insensitive = self.get_object::<CheckButton>(ID_CASE_SENSITIVE);

            {
                let callback = callback.clone();
                pattern_entry.connect_changed(move |_| callback.borrow_mut()());
            }
            {
                let callback = callback.clone();
                regexp_supported.connect_toggled(move |_| callback.borrow_mut()());
            }
            {
                let callback = callback.clone();
                replacement_entry.connect_changed(move |_| callback.borrow_mut()());
            }
            {
                let callback = callback.clone();
                case_insensitive.connect_toggled(move |_| callback.borrow_mut()());
            }
        }
    }

    fn get_object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.get_object(name).unwrap()
    }
}

impl ProviderCommon for ReplaceRenamer {
    fn get_panel(&self) -> Container {
        self.get_object::<Container>(ID_REPLACE_RENAMER_PANEL)
    }

    fn apply_replacement(&self, files: &[(String, String)]) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| (file_name.clone(), dir_name.clone()))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use glib::bitflags::_core::sync::atomic::Ordering::SeqCst;
    use gtk::WindowBuilder;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_replace_renamer_callback() {
        gtk::init().unwrap();
        let counter = Arc::new(AtomicUsize::new(0));
        let replace_renamer = {
            let counter = counter.clone();
            ReplaceRenamer::new(Some(Rc::new(RefCell::new(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            }))))
        };
        let pattern_entry = replace_renamer.get_object::<Entry>(ID_PATTERN_ENTRY);
        let regexp_supported = replace_renamer.get_object::<CheckButton>(ID_REGEXP_SUPPORTED);
        let replacement_entry = replace_renamer.get_object::<Entry>(ID_REPLACEMENT_ENTRY);
        let case_insensitive = replace_renamer.get_object::<CheckButton>(ID_CASE_SENSITIVE);

        WindowBuilder::new()
            .child(&replace_renamer.get_panel())
            .build()
            .show_all();

        counter.store(0, SeqCst);
        gtk_test::enter_keys(&pattern_entry, "from");
        assert_eq!(counter.load(Ordering::SeqCst), "from".len());

        counter.store(0, SeqCst);
        gtk_test::click(&regexp_supported);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        gtk_test::click(&regexp_supported);
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        counter.store(0, SeqCst);
        gtk_test::enter_keys(&replacement_entry, "to");
        assert_eq!(counter.load(Ordering::SeqCst), "to".len());

        counter.store(0, SeqCst);
        gtk_test::click(&case_insensitive);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        gtk_test::click(&case_insensitive);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_replace_renamer_apply_replacement() {
        gtk::init().unwrap();
        let replace_renamer = ReplaceRenamer::new(None);

        let replacement = replace_renamer
            .apply_replacement(&[("file_name_from".to_string(), "dirname".to_string())]);

        assert_eq!(
            replacement.as_slice(),
            &[("file_name_from".to_string(), "dirname".to_string()),]
        );
    }
}
