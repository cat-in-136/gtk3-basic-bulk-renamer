use gio::{ActionMapExt, ApplicationExt, SimpleAction};
use gtk::prelude::*;
use gtk::ListStore;
use gtk::{Application, TreeView};
use gtk::{ApplicationWindow, Builder, GtkWindowExt};
use std::path::{Path, PathBuf};

const ID_ADD_BUTTON: &'static str = "add-button";
const ID_CLEAR_BUTTON: &'static str = "clear-button";
const ID_EXECUTE_BUTTON: &'static str = "execute-button";
const ID_FILE_LIST: &'static str = "file-list";
const ID_FILE_LIST_STORE: &'static str = "file-list-store";
const ID_HEADERBAR: &'static str = "headerbar";
const ID_MAIN_WINDOW: &'static str = "main-window";
const ID_NOTEBOOK: &'static str = "notebook";
const ID_REMOVE_BUTTON: &'static str = "remove-button";

pub(crate) struct Window {
    builder: Builder,
}

impl Window {
    pub fn new(app: &Application) -> Self {
        let builder = Builder::from_string(include_str!("window.glade"));
        let window = Self { builder };

        window.init_action(app);

        let main_window = window.main_window();
        main_window.set_application(Some(app));

        window
    }

    fn get_cloned_object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        let object: T = self.builder.get_object(name).unwrap();
        object.clone()
    }

    fn init_action(&self, _app: &Application) {
        let main_window = self.main_window();
        let clear_action = SimpleAction::new("clear-action", None);
        {
            let file_list_store = self.get_cloned_object::<ListStore>(ID_FILE_LIST_STORE);
            clear_action.connect_activate(move |_, _| {
                file_list_store.clear();
            });
        }
        main_window.add_action(&clear_action);
    }

    pub fn set_files(&self, paths: &[PathBuf]) {
        let file_list_store = self.get_cloned_object::<ListStore>(ID_FILE_LIST_STORE);
        file_list_store.clear();

        for path in paths.iter() {
            let iter = file_list_store.append();
            file_list_store.set(
                &iter,
                &[0, 1],
                &[&path.display().to_string(), &path.display().to_string()],
            );
        }
    }

    pub fn main_window(&self) -> ApplicationWindow {
        self.get_cloned_object(ID_MAIN_WINDOW)
    }
}
