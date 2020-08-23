use crate::error::Error;
use crate::observer::Observer;
use crate::utils::{list_store_data_iter, value2string};
use crate::win::provider::{Provider, RenamerType};
use gio::{ActionMapExt, SimpleAction};
use gtk::prelude::*;
use gtk::{Application, LabelBuilder, Notebook, TreeView};
use gtk::{ApplicationWindow, Builder, GtkWindowExt};
use gtk::{FileChooserAction, FileChooserDialogBuilder, ListStore, ResponseType};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use strum::IntoEnumIterator;

const ACTION_ADD: &'static str = "add-action";
const ACTION_REMOVE: &'static str = "remove-action";
const ACTION_CLEAR: &'static str = "clear-action";

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
    provider: Rc<Provider>,
}

impl Window {
    pub fn new<P: IsA<Application>>(app: Option<&P>) -> Self {
        let builder = Builder::from_string(include_str!("window.glade"));
        let provider = Rc::new(Provider::new());
        let window = Self { builder, provider };

        window.init_actions();
        window.init_signals();
        window.init_provider_panels();

        let main_window = window.main_window();
        main_window.set_application(app);

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

    fn init_actions(&self) {
        let main_window = self.main_window();
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let selection = self.get_object::<TreeView>(ID_FILE_LIST).get_selection();

        let renamer_change_observer = Rc::new(RenamerChangeObserver {
            builder: self.builder.clone(),
            provider: self.provider.clone(),
        });
        self.provider.attach_change(renamer_change_observer.clone());

        let add_action = SimpleAction::new(ACTION_ADD, None);
        {
            generate_clones!(main_window, file_list_store, renamer_change_observer);
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
                    renamer_change_observer.update(&()).unwrap(); // TODO unwrap
                }
            });
        }
        main_window.add_action(&add_action);

        let remove_action = SimpleAction::new(ACTION_REMOVE, None);
        {
            generate_clones!(file_list_store, selection);
            remove_action.connect_activate(move |_, _| {
                selection.selected_foreach(|_, _, iter| {
                    file_list_store.remove(iter);
                });
            });
        }
        main_window.add_action(&remove_action);

        let clear_action = SimpleAction::new(ACTION_CLEAR, None);
        {
            generate_clones!(file_list_store);
            clear_action.connect_activate(move |_, _| {
                file_list_store.clear();
            });
        }
        main_window.add_action(&clear_action);
    }

    fn init_signals(&self) {
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let selection = self.get_object::<TreeView>(ID_FILE_LIST).get_selection();

        let update_action_enabled = {
            generate_clones!(file_list_store, selection);
            let remove_action = self.get_simple_action(ACTION_REMOVE);
            let clear_action = self.get_simple_action(ACTION_CLEAR);
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

    fn init_provider_panels(&self) {
        let notebook = self.get_object::<Notebook>(ID_NOTEBOOK);
        for renamer_type in RenamerType::iter() {
            let renamer = self.provider.renamer_of(renamer_type);
            let tab_label = LabelBuilder::new().label(renamer_type.label()).build();
            let panel = renamer.get_panel();
            notebook.append_page(&panel, Some(&tab_label));
        }
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

struct RenamerChangeObserver {
    builder: Builder,
    provider: Rc<Provider>,
}
impl RenamerChangeObserver {
    fn get_object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.get_object(name).unwrap()
    }
}

impl Observer<(), Error> for RenamerChangeObserver {
    fn update(&self, arg: &()) -> Result<(), Error> {
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let notebook = self.get_object::<Notebook>(ID_NOTEBOOK);
        let provider = self.provider.clone();

        let data = list_store_data_iter(&file_list_store)
            .map(|row| (value2string(&row[0]), value2string(&row[2])))
            .collect::<Vec<_>>();

        if let (Some(renamer_type), Some(iter)) = (
            notebook
                .get_current_page()
                .and_then(|v| RenamerType::iter().nth(v as usize)),
            file_list_store.get_iter_first(),
        ) {
            provider
                .renamer_of(renamer_type)
                .apply_replacement(data.as_slice())
                .and_then(|replacements| {
                    for (new_file_name, _) in replacements {
                        file_list_store.set(&iter, &[1], &[&new_file_name]);
                        file_list_store.iter_next(&iter);
                    }
                    Ok(())
                })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use gio::ActionExt;

    #[test]
    fn test_init_signals() {
        gtk::init().unwrap();

        let win = Window::new::<Application>(None);
        win.main_window().show_all();

        assert_eq!(
            win.get_object::<ListStore>(ID_FILE_LIST_STORE)
                .iter_n_children(None),
            0
        );

        assert_eq!(win.get_simple_action(ACTION_ADD).get_enabled(), true);
        assert_eq!(win.get_simple_action(ACTION_REMOVE).get_enabled(), false);
        assert_eq!(win.get_simple_action(ACTION_CLEAR).get_enabled(), false);

        win.set_files(&[PathBuf::from("test")]);
        assert_eq!(
            win.get_object::<ListStore>(ID_FILE_LIST_STORE)
                .iter_n_children(None),
            1
        );

        assert_eq!(win.get_simple_action(ACTION_ADD).get_enabled(), true);
        assert_eq!(win.get_simple_action(ACTION_REMOVE).get_enabled(), false);
        assert_eq!(win.get_simple_action(ACTION_CLEAR).get_enabled(), true);

        gtk_test::click(&win.get_object::<TreeView>(ID_FILE_LIST));
        assert_eq!(
            win.get_object::<TreeView>(ID_FILE_LIST)
                .get_selection()
                .count_selected_rows(),
            1
        );

        assert_eq!(win.get_simple_action(ACTION_ADD).get_enabled(), true);
        assert_eq!(win.get_simple_action(ACTION_REMOVE).get_enabled(), true);
        assert_eq!(win.get_simple_action(ACTION_CLEAR).get_enabled(), true);
    }

    #[test]
    fn test_set_files() {
        gtk::init().unwrap();

        let win = Window::new::<Application>(None);
        // win.main_window().show_all();

        let file_list_store = win.get_object::<ListStore>(ID_FILE_LIST_STORE);
        assert_eq!(file_list_store.iter_n_children(None), 0);

        win.set_files(&[PathBuf::from("test"), PathBuf::from("/test2")]);
        assert_eq!(file_list_store.iter_n_children(None), 2);

        let iter = file_list_store.iter_nth_child(None, 0).unwrap();
        assert_eq!(
            file_list_store.get_value(&iter, 0).get(),
            Ok(Some(String::from("test")))
        );
        assert_eq!(
            file_list_store.get_value(&iter, 1).get(),
            Ok(Some(String::from("test")))
        );
        assert_eq!(
            file_list_store.get_value(&iter, 2).get(),
            Ok(Some(String::from("")))
        );
        let iter = file_list_store.iter_nth_child(None, 1).unwrap();
        assert_eq!(
            file_list_store.get_value(&iter, 0).get(),
            Ok(Some(String::from("test2")))
        );
        assert_eq!(
            file_list_store.get_value(&iter, 1).get(),
            Ok(Some(String::from("test2")))
        );
        assert_eq!(
            file_list_store.get_value(&iter, 2).get(),
            Ok(Some(String::from("/")))
        );
    }
}
