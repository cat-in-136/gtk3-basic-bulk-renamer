use crate::win::provider::ProviderCommon;
use gtk::prelude::*;
use gtk::{Builder, CheckButton, Container, Entry};
use std::cell::RefCell;
use std::rc::Rc;

const ID_REPLACE_RENAMER_PANEL: &'static str = "replace-renamer-panel";
const ID_PATTERN_ENTRY: &'static str = "pattern-entry";
const ID_REGEXP_SUPPORTED: &'static str = "regexp-supported";
const ID_REPLACEMENT_ENTRY: &'static str = "replacement-entry";
const ID_CASE_SENSITIVE: &'static str = "case-sensitive";

pub struct ReplaceRenamer {
    builder: Builder,
    callback: Option<Rc<RefCell<fn()>>>,
}

impl ReplaceRenamer {
    pub fn new(callback: Option<Rc<RefCell<fn()>>>) -> Self {
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
                regexp_supported.connect_state_flags_changed(move |_, _| callback.borrow_mut()());
            }
            {
                let callback = callback.clone();
                replacement_entry.connect_changed(move |_| callback.borrow_mut()());
            }
            {
                let callback = callback.clone();
                case_insensitive.connect_state_flags_changed(move |_, _| callback.borrow_mut()());
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

    fn apply_replacement(&self, files: &[(String, String)]) -> &[(String, String)] {
        unimplemented!()
    }
}
