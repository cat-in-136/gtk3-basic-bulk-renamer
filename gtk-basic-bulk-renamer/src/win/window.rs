use gtk::prelude::*;
use gtk::{Application, TreeView};
use gtk::{ApplicationWindow, Builder, GtkWindowExt};
use std::path::{Path, PathBuf};
use gtk::ListStore;

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

        let main_window = window.main_window();
        main_window.set_application(Some(app));

        window
    }

    pub fn set_files(&self, paths: &[PathBuf]) {
        let file_list_store: ListStore = self.builder.get_object(ID_FILE_LIST_STORE).unwrap();
        file_list_store.clear();

        for path in paths.iter() {
            let iter = file_list_store.append();
            file_list_store.set(&iter, &[0, 1], &[
                &path.display().to_string(),
                &path.display().to_string(),
            ]);
        }

    }

    pub fn main_window(&self) -> ApplicationWindow {
        self.builder.get_object(ID_MAIN_WINDOW).unwrap()
    }
}


