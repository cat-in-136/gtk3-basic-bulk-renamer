use crate::basic_bulk_renamer::{BulkRename, RenameError, RenameOverwriteMode};
use crate::error::Error;
use crate::utils::get_path_from_selection_data;
use crate::utils::Observer;
use crate::win::file_list::{
    add_files_to_file_list, apply_renamer_to_file_list, get_files_from_file_list,
    reset_renaming_of_file_list, set_files_to_file_list, RenamerTarget,
};
use crate::win::provider::{Provider, RenamerObserverArg, RenamerType};
use crate::win::resource::{init_resource, resource_path};
use gdk::DragAction;
use gio::prelude::*;
use gio::SimpleAction;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Builder, ButtonsType, ComboBoxText, DestDefaults,
    FileChooserAction, FileChooserDialog, ListStore, MessageDialog, MessageType, ResponseType,
    Stack, TargetEntry, TargetFlags, TreeView,
};
use std::path::PathBuf;
use std::rc::Rc;
use std::str::FromStr;
use strum::IntoEnumIterator;

const ACTION_ADD: &'static str = "add-action";
const ACTION_REMOVE: &'static str = "remove-action";
const ACTION_CLEAR: &'static str = "clear-action";
const ACTION_EXECUTE: &'static str = "execute-action";

const ID_FILE_LIST: &'static str = "file-list";
const ID_FILE_LIST_STORE: &'static str = "file-list-store";
const ID_MAIN_WINDOW: &'static str = "main-window";
const ID_RENAME_TARGET_COMBO_BOX: &'static str = "rename-target-combo-box";
const ID_PROVIDER_STACK: &'static str = "provider-stack";
const ID_PROVIDER_SWITCHER_COMBO_BOX: &'static str = "provider-switcher-combo-box";

pub(crate) struct Window {
    builder: Builder,
    provider: Rc<Provider>,
}

impl Window {
    pub fn new<P: IsA<Application>>(app: Option<&P>) -> Self {
        init_resource();

        let builder = Builder::from_resource(&resource_path("window.glade"));
        let provider = Rc::new(Provider::new());
        let window = Self { builder, provider };

        window.init_actions_signals();
        window.init_provider_panels();

        let main_window = window.main_window();
        main_window.set_application(app);

        window
    }

    fn object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.object(name).unwrap()
    }

    #[cfg(test)]
    fn simple_action(&self, name: &str) -> SimpleAction {
        self.main_window()
            .lookup_action(name)
            .unwrap()
            .downcast::<_>()
            .unwrap()
    }

    fn init_actions_signals(&self) {
        let main_window = self.main_window();
        let file_list_store = self.object::<ListStore>(ID_FILE_LIST_STORE);
        let file_list = self.object::<TreeView>(ID_FILE_LIST);
        let selection = file_list.clone().selection();
        let rename_target_combo_box = self.object::<ComboBoxText>(ID_RENAME_TARGET_COMBO_BOX);
        let provider_stack = self.object::<Stack>(ID_PROVIDER_STACK);

        let renamer_change_observer = Rc::new(RenamerChangeObserver {
            builder: self.builder.clone(),
            provider: self.provider.clone(),
        });
        self.provider.attach_change(renamer_change_observer.clone());

        let add_action = SimpleAction::new(ACTION_ADD, None);
        add_action.connect_activate(glib::clone!(
            @weak main_window,
            @weak file_list_store,
            @weak provider_stack,
            @weak renamer_change_observer => move |_, _| {
            let dialog = FileChooserDialog::builder()
                .title("Add")
                .application(&main_window.application().unwrap())
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
                let paths = dialog.filenames();
                add_files_to_file_list(&file_list_store, &paths);

                let renamer_type = provider_stack
                    .visible_child_name()
                    .and_then(|v| RenamerType::from_str(v.as_str()).ok())
                    .unwrap_or(RenamerType::Replace);
                renamer_change_observer
                    .update(&(renamer_type, ()))
                    .unwrap_or_else(|_| {
                        reset_renaming_of_file_list(&file_list_store);
                    });
            }
        }));
        main_window.add_action(&add_action);

        let remove_action = SimpleAction::new(ACTION_REMOVE, None);
        remove_action.connect_activate(
            glib::clone!(@weak file_list_store, @weak selection => move |_, _| {
                selection.selected_foreach(|_, _, iter| {
                    file_list_store.remove(iter);
                });
            }),
        );
        main_window.add_action(&remove_action);

        let clear_action = SimpleAction::new(ACTION_CLEAR, None);
        clear_action.connect_activate(glib::clone!(@weak file_list_store => move |_, _| {
            file_list_store.clear();
        }));
        main_window.add_action(&clear_action);

        let execute_action = SimpleAction::new(ACTION_EXECUTE, None);
        execute_action.connect_activate(glib::clone!(
            @weak main_window,
            @weak file_list_store,
            @weak provider_stack,
            @weak renamer_change_observer => move |_, _| {
            let files = get_files_from_file_list(&file_list_store).collect::<Vec<_>>();
            let mut renamer = BulkRename::new(files.clone());
            renamer
                .execute(RenameOverwriteMode::Error)
                .map_err(|e| Error::Rename(e))
                .and_then(|_| {
                    let new_files = files.iter().map(|v| v.1.clone()).collect::<Vec<_>>();
                    file_list_store.clear();
                    add_files_to_file_list(&file_list_store, &new_files);
                    let renamer_type = provider_stack
                        .visible_child_name()
                        .and_then(|v| RenamerType::from_str(v.as_str()).ok())
                        .unwrap_or(RenamerType::Replace);
                    renamer_change_observer.update(&(renamer_type, ()))
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

                    let dialog = MessageDialog::builder()
                        .application(&main_window.application().unwrap())
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
        }));
        main_window.add_action(&execute_action);

        selection.connect_changed(glib::clone!(
            @weak file_list_store,
            @weak file_list,
            @weak selection,
            @weak remove_action,
            @weak clear_action,
            @weak execute_action => move |_| {
            let file_list_store_count = file_list_store.iter_n_children(None);
            if file_list_store_count == 0 {
                file_list.columns_autosize();
            }
            remove_action.set_enabled(selection.count_selected_rows() > 0);
            clear_action.set_enabled(file_list_store_count > 0);
            execute_action.set_enabled(file_list_store_count > 0);
        }));
        file_list_store.connect_row_inserted(glib::clone!(@weak selection => move |_, _, _| {
            selection.emit_by_name::<()>("changed", &[]);
        }));
        file_list_store.connect_row_deleted(glib::clone!(@weak selection => move |_, _| {
            selection.emit_by_name::<()>("changed", &[]);
        }));
        selection.emit_by_name::<()>("changed", &[]);

        provider_stack.connect_visible_child_notify(glib::clone!(@weak file_list_store, @weak renamer_change_observer => move |provider_stack| {
                let renamer_type = provider_stack
                    .visible_child_name()
                    .and_then(|v| RenamerType::from_str(v.as_str()).ok())
                    .unwrap_or(RenamerType::Replace);
                renamer_change_observer
                    .update(&(renamer_type, ()))
                    .unwrap_or_else(|_| {
                        reset_renaming_of_file_list(&file_list_store);
                    });
            }));
        rename_target_combo_box.connect_changed(glib::clone!(@weak file_list_store, @weak provider_stack, @weak renamer_change_observer => move |_| {
                let renamer_type = provider_stack
                    .visible_child_name()
                    .and_then(|v| RenamerType::from_str(v.as_str()).ok())
                    .unwrap_or(RenamerType::Replace);
                renamer_change_observer
                    .update(&(renamer_type, ()))
                    .unwrap_or_else(|_| {
                        reset_renaming_of_file_list(&file_list_store);
                    });
            }));

        let dnd_target_entries = &[
            TargetEntry::new("STRING", TargetFlags::empty(), 0),
            TargetEntry::new("text/plain", TargetFlags::empty(), 0),
            TargetEntry::new("text/uri-list", TargetFlags::empty(), 0),
        ];
        file_list.drag_dest_set(DestDefaults::ALL, dnd_target_entries, DragAction::COPY);

        file_list.connect_drag_data_received(glib::clone!(@weak renamer_change_observer => move |_file_list, _c, _x, _y, sel_data, _info, _time| {
                    let paths = get_path_from_selection_data(&sel_data);
                    add_files_to_file_list(&file_list_store, &paths);
                    let renamer_type = provider_stack
                        .visible_child_name()
                        .and_then(|v| RenamerType::from_str(v.as_str()).ok())
                        .unwrap_or(RenamerType::Replace);
                    renamer_change_observer
                        .update(&(renamer_type, ()))
                        .unwrap_or_else(|_| {
                            reset_renaming_of_file_list(&file_list_store);
                        });
                }));
    }

    fn init_provider_panels(&self) {
        let provider_stack = self.object::<Stack>(ID_PROVIDER_STACK);
        let provider_switcher_combo_box =
            self.object::<ComboBoxText>(ID_PROVIDER_SWITCHER_COMBO_BOX);
        for renamer_type in RenamerType::iter() {
            let name = renamer_type.into();
            let title = renamer_type.label();
            let renamer = self.provider.renamer_of(renamer_type);
            let panel = renamer.get_panel();

            provider_stack.add_titled(&panel, name, title);
            provider_switcher_combo_box.append(Some(name), title);
        }
        provider_switcher_combo_box.set_active_id(Some(RenamerType::Replace.into()));

        provider_switcher_combo_box.connect_changed(
            glib::clone!(@weak provider_stack => move |provider_switcher_combo_box| {
                if let Some(active_id) = provider_switcher_combo_box.active_id() {
                    provider_stack.set_visible_child_name(active_id.as_str());
                }
            }),
        );
    }

    pub fn set_files(&self, paths: &[PathBuf]) {
        let file_list_store = self.object::<ListStore>(ID_FILE_LIST_STORE);
        set_files_to_file_list(&file_list_store, paths);
    }

    pub fn main_window(&self) -> ApplicationWindow {
        self.object(ID_MAIN_WINDOW)
    }
}

struct RenamerChangeObserver {
    builder: Builder,
    provider: Rc<Provider>,
}
impl RenamerChangeObserver {
    fn object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.object(name).unwrap()
    }
}

impl Observer<RenamerObserverArg, Error> for RenamerChangeObserver {
    fn update(&self, arg: &RenamerObserverArg) -> Result<(), Error> {
        let (renamer_type, _) = *arg;
        let file_list_store = self.object::<ListStore>(ID_FILE_LIST_STORE);
        let provider = self.provider.clone();
        let renamer = provider.renamer_of(renamer_type);
        let target = self
            .object::<ComboBoxText>(ID_RENAME_TARGET_COMBO_BOX)
            .active_id()
            .and_then(|id| RenamerTarget::from_str(id.as_str()).ok())
            .unwrap_or(RenamerTarget::All);
        apply_renamer_to_file_list(&file_list_store, target, renamer)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_synced;

    #[test]
    fn test_init_actions_signals() {
        test_synced(move || {
            let win = Window::new::<Application>(None);
            win.main_window().show_all();

            assert_eq!(
                win.object::<ListStore>(ID_FILE_LIST_STORE)
                    .iter_n_children(None),
                0
            );

            assert_eq!(win.simple_action(ACTION_ADD).is_enabled(), true);
            assert_eq!(win.simple_action(ACTION_REMOVE).is_enabled(), false);
            assert_eq!(win.simple_action(ACTION_CLEAR).is_enabled(), false);
            assert_eq!(win.simple_action(ACTION_EXECUTE).is_enabled(), false);

            win.set_files(&[PathBuf::from("test")]);
            assert_eq!(
                win.object::<ListStore>(ID_FILE_LIST_STORE)
                    .iter_n_children(None),
                1
            );

            assert_eq!(win.simple_action(ACTION_ADD).is_enabled(), true);
            assert_eq!(win.simple_action(ACTION_REMOVE).is_enabled(), false);
            assert_eq!(win.simple_action(ACTION_CLEAR).is_enabled(), true);
            assert_eq!(win.simple_action(ACTION_EXECUTE).is_enabled(), true);

            gtk_test::click(&win.object::<TreeView>(ID_FILE_LIST));
            assert_eq!(
                win.object::<TreeView>(ID_FILE_LIST)
                    .selection()
                    .count_selected_rows(),
                1
            );

            assert_eq!(win.simple_action(ACTION_ADD).is_enabled(), true);
            assert_eq!(win.simple_action(ACTION_REMOVE).is_enabled(), true);
            assert_eq!(win.simple_action(ACTION_CLEAR).is_enabled(), true);
            assert_eq!(win.simple_action(ACTION_EXECUTE).is_enabled(), true);
        });
    }
}
