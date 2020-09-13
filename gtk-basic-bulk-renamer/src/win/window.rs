use crate::error::Error;
use crate::observer::Observer;
use crate::utils::get_path_from_selection_data;
use crate::win::file_list::{
    add_files_to_file_list, apply_renamer_to_file_list, get_files_from_file_list,
    reset_renaming_of_file_list, set_files_to_file_list,
};
use crate::win::provider::{Provider, RenamerType};
use basic_bulk_renamer::{BulkRename, RenameError, RenameOverwriteMode};
use gdk::DragAction;
use gio::{ActionMapExt, SimpleAction};
use gtk::prelude::*;
use gtk::{
    Application, ButtonsType, DestDefaults, LabelBuilder, MessageDialogBuilder, MessageType,
    Notebook, TargetEntry, TargetFlags, TreeView,
};
use gtk::{ApplicationWindow, Builder, GtkWindowExt};
use gtk::{FileChooserAction, FileChooserDialogBuilder, ListStore, ResponseType};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use strum::IntoEnumIterator;

const ACTION_ADD: &'static str = "add-action";
const ACTION_REMOVE: &'static str = "remove-action";
const ACTION_CLEAR: &'static str = "clear-action";
const ACTION_EXECUTE: &'static str = "execute-action";

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

        window.init_actions_signals();
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

    fn init_actions_signals(&self) {
        let main_window = self.main_window();
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let file_list = self.get_object::<TreeView>(ID_FILE_LIST);
        let selection = file_list.clone().get_selection();
        let notebook = self.get_object::<Notebook>(ID_NOTEBOOK);

        let renamer_change_observer = Rc::new(RenamerChangeObserver {
            builder: self.builder.clone(),
            provider: self.provider.clone(),
        });
        self.provider.attach_change(renamer_change_observer.clone());

        let add_action = SimpleAction::new(ACTION_ADD, None);
        {
            generate_clones!(
                main_window,
                file_list_store,
                notebook,
                renamer_change_observer
            );
            add_action.connect_activate(move |_, _| {
                let dialog = FileChooserDialogBuilder::new()
                    .title("Add")
                    .application(&main_window.get_application().unwrap())
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
                    add_files_to_file_list(&file_list_store, &paths);

                    let renamer_type = Self::get_renamer_type_from_notebook(&notebook);
                    renamer_change_observer
                        .update(&(renamer_type))
                        .unwrap_or_else(|_| {
                            reset_renaming_of_file_list(&file_list_store);
                        });
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

        let execute_action = SimpleAction::new(ACTION_EXECUTE, None);
        {
            generate_clones!(
                main_window,
                file_list_store,
                notebook,
                renamer_change_observer
            );
            execute_action.connect_activate(move |_, _| {
                let files = get_files_from_file_list(&file_list_store).collect::<Vec<_>>();
                let mut renamer = BulkRename::new(files.clone());
                renamer
                    .execute(RenameOverwriteMode::Error)
                    .map_err(|e| Error::Rename(e))
                    .and_then(|_| {
                        let new_files = files.iter().map(|v| v.1.clone()).collect::<Vec<_>>();
                        file_list_store.clear();
                        add_files_to_file_list(&file_list_store, &new_files);
                        let renamer_type = Self::get_renamer_type_from_notebook(&notebook);
                        renamer_change_observer.update(&(renamer_type))
                    })
                    .or_else(|e| {
                        let undo_error = renamer
                            .undo_bulk_rename()
                            .ok_or(RenameError::IllegalOperation)
                            .and_then(|mut undo_renamer| {
                                undo_renamer.execute(RenameOverwriteMode::Error)
                            });
                        let detailed_message = format!(
                            "{}\n{}",
                            e.to_string(),
                            match undo_error {
                                Ok(_) => "Rename is not applied".to_string(),
                                Err(undo_rename_error) => format!(
                                    "Rename is interrupted: {}",
                                    undo_rename_error.to_string()
                                ),
                            }
                        );

                        let dialog = MessageDialogBuilder::new()
                            .application(&main_window.get_application().unwrap())
                            .buttons(ButtonsType::Ok)
                            .message_type(MessageType::Error)
                            .text("Failed to rename")
                            .secondary_text(detailed_message.as_str())
                            .build();
                        dialog.run();
                        dialog.close();
                        Err(())
                    })
                    .unwrap_or_default();
            });
        }
        main_window.add_action(&execute_action);

        let update_action_enabled = {
            generate_clones!(
                file_list_store,
                selection,
                remove_action,
                clear_action,
                execute_action
            );
            Rc::new(RefCell::new(move || {
                remove_action.set_enabled(selection.count_selected_rows() > 0);
                clear_action.set_enabled(file_list_store.iter_n_children(None) > 0);
                execute_action.set_enabled(file_list_store.iter_n_children(None) > 0);
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

        {
            generate_clones!(file_list_store, notebook, renamer_change_observer);
            notebook.connect_switch_page(move |_, _, page_id| {
                let renamer_type = RenamerType::iter()
                    .nth(page_id as usize)
                    .unwrap_or(RenamerType::Replace);
                renamer_change_observer
                    .update(&(renamer_type))
                    .unwrap_or_else(|_| {
                        reset_renaming_of_file_list(&file_list_store);
                    });
            });
        }

        let dnd_target_entries = &[
            TargetEntry::new("STRING", TargetFlags::empty(), 0),
            TargetEntry::new("text/plain", TargetFlags::empty(), 0),
            TargetEntry::new("text/uri-list", TargetFlags::empty(), 0),
        ];
        file_list.drag_dest_set(DestDefaults::ALL, dnd_target_entries, DragAction::COPY);
        {
            generate_clones!(renamer_change_observer);
            file_list.connect_drag_data_received(
                move |_file_list, _c, _x, _y, sel_data, _info, _time| {
                    let paths = get_path_from_selection_data(&sel_data);
                    add_files_to_file_list(&file_list_store, &paths);
                    let renamer_type = Self::get_renamer_type_from_notebook(&notebook);
                    renamer_change_observer
                        .update(&(renamer_type))
                        .unwrap_or_else(|_| {
                            reset_renaming_of_file_list(&file_list_store);
                        });
                },
            );
        }
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

    pub fn set_files(&self, paths: &[PathBuf]) {
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        set_files_to_file_list(&file_list_store, paths);
    }

    pub fn main_window(&self) -> ApplicationWindow {
        self.get_object(ID_MAIN_WINDOW)
    }

    fn get_renamer_type_from_notebook(notebook: &Notebook) -> RenamerType {
        notebook
            .get_current_page()
            .and_then(|v| RenamerType::iter().nth(v as usize))
            .unwrap_or(RenamerType::Replace)
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

impl Observer<(RenamerType), Error> for RenamerChangeObserver {
    fn update(&self, arg: &(RenamerType)) -> Result<(), Error> {
        let renamer_type = *arg;
        let file_list_store = self.get_object::<ListStore>(ID_FILE_LIST_STORE);
        let provider = self.provider.clone();
        let renamer = provider.renamer_of(renamer_type);
        apply_renamer_to_file_list(&file_list_store, renamer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use gio::ActionExt;

    #[test]
    fn test_init_actions_signals() {
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
        assert_eq!(win.get_simple_action(ACTION_EXECUTE).get_enabled(), false);

        win.set_files(&[PathBuf::from("test")]);
        assert_eq!(
            win.get_object::<ListStore>(ID_FILE_LIST_STORE)
                .iter_n_children(None),
            1
        );

        assert_eq!(win.get_simple_action(ACTION_ADD).get_enabled(), true);
        assert_eq!(win.get_simple_action(ACTION_REMOVE).get_enabled(), false);
        assert_eq!(win.get_simple_action(ACTION_CLEAR).get_enabled(), true);
        assert_eq!(win.get_simple_action(ACTION_EXECUTE).get_enabled(), true);

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
        assert_eq!(win.get_simple_action(ACTION_EXECUTE).get_enabled(), true);
    }
}
