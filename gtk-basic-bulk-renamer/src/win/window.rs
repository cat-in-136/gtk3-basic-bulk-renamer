use gio::{ActionMapExt, ApplicationExt, SimpleAction};
use gtk::prelude::*;
use gtk::{Application, TreeView};
use gtk::{ApplicationWindow, Builder, GtkWindowExt};
use gtk::{FileChooserAction, FileChooserDialogBuilder, ListStore, ResponseType};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const ID_ADD_BUTTON: &'static str = "add-button";
const ID_CLEAR_BUTTON: &'static str = "clear-button";
const ID_EXECUTE_BUTTON: &'static str = "execute-button";
const ID_FILE_LIST: &'static str = "file-list";
const ID_FILE_LIST_STORE: &'static str = "file-list-store";
const ID_HEADERBAR: &'static str = "headerbar";
const ID_MAIN_WINDOW: &'static str = "main-window";
const ID_NOTEBOOK: &'static str = "notebook";
const ID_REMOVE_BUTTON: &'static str = "remove-button";

macro_rules! generate_clones {
    ($($n:ident),+) => (
        $( let $n = $n.clone(); )+
    )
}

pub(crate) struct Window {
    builder: Builder,
}

impl Window {
    pub fn new(app: &Application) -> Self {
        let builder = Builder::from_string(include_str!("window.glade"));
        let window = Self { builder };

        window.init_actions(app);
        window.init_signals(app);

        let main_window = window.main_window();
        main_window.set_application(Some(app));

        window
    }

    fn get_object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.get_object(name).unwrap()
    }

    fn get_simple_action(&self, name: &str) -> SimpleAction {
        self.main_window()
            .lookup_action(name)
            .unwrap()
            .downcast::<_>()
            .unwrap()
    }

    fn init_actions(&self, _app: &Application) {
        let main_window = self.main_window();
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let selection = self.get_object::<TreeView>(ID_FILE_LIST).get_selection();

        let add_action = SimpleAction::new("add-action", None);
        {
            generate_clones!(main_window, file_list_store);
            add_action.connect_activate(move |_, _| {
                let dialog = FileChooserDialogBuilder::new()
                    .title("Add")
                    .parent(&main_window)
                    .select_multiple(true)
                    .mnemonics_visible(true)
                    .action(FileChooserAction::Open)
                    .build();
                dialog.add_buttons(&[
                    ("_Cancel", ResponseType::Cancel),
                    ("_OK", ResponseType::Accept),
                ]);
                let result = dialog.run();
                dialog.close();

                if result == ResponseType::Accept {
                    let paths = dialog.get_filenames();
                    Self::add_files_to(&file_list_store, &paths);
                }
            });
        }
        main_window.add_action(&add_action);

        let remove_action = SimpleAction::new("remove-action", None);
        {
            generate_clones!(file_list_store, selection);
            remove_action.connect_activate(move |_, _| {
                selection.selected_foreach(|_, _, iter| {
                    file_list_store.remove(iter);
                });
            });
        }
        main_window.add_action(&remove_action);

        let clear_action = SimpleAction::new("clear-action", None);
        {
            generate_clones!(file_list_store);
            clear_action.connect_activate(move |_, _| {
                file_list_store.clear();
            });
        }
        main_window.add_action(&clear_action);
    }

    fn init_signals(&self, _app: &Application) {
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let selection = self.get_object::<TreeView>(ID_FILE_LIST).get_selection();

        let update_action_enabled = {
            generate_clones!(file_list_store, selection);
            let remove_action = self.get_simple_action("remove-action");
            let clear_action = self.get_simple_action("clear-action");
            Rc::new(RefCell::new(move || {
                remove_action.set_enabled(selection.count_selected_rows() > 0);
                clear_action.set_enabled(file_list_store.iter_n_children(None) > 0);
            }))
        };

        {
            generate_clones!(update_action_enabled);
            selection.connect_changed(move |_| update_action_enabled.borrow_mut()());
        }
        {
            generate_clones!(update_action_enabled);
            file_list_store
                .connect_row_inserted(move |_, _, _| update_action_enabled.borrow_mut()());
        }
        {
            generate_clones!(update_action_enabled);
            file_list_store.connect_row_deleted(move |_, _| update_action_enabled.borrow_mut()());
        }
        update_action_enabled.clone().borrow_mut()();
    }

    fn add_files_to(file_list_store: &ListStore, paths: &[PathBuf]) {
        for path in paths.iter() {
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
                .to_string();
            let new_name = name.clone();
            let parent = path.parent().unwrap().display().to_string();

            let iter = file_list_store.append();
            file_list_store.set(&iter, &[0, 1, 2], &[&name, &new_name, &parent]);
        }
    }

    pub fn set_files(&self, paths: &[PathBuf]) {
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        file_list_store.clear();
        Self::add_files_to(&file_list_store, paths);
    }

    pub fn main_window(&self) -> ApplicationWindow {
        self.get_object(ID_MAIN_WINDOW)
    }
}
