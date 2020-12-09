use crate::error::Error;
use crate::utils::{
    split_file_at_dot, BulkTextReplacement, InsertPosition, TextCharPosition, TextInsertOrOverwrite,
};
use crate::utils::{Observer, SubjectImpl};
use crate::win::provider::{Renamer, RenamerObserverArg, RenamerTarget, RenamerType};
use gtk::prelude::*;
use gtk::{Builder, ComboBoxText, Container, Entry, SpinButton};
use std::convert::TryFrom;
use std::rc::Rc;
use std::vec::IntoIter;

const ID_INSERT_OVERWRITE_RENAMER_PANEL: &'static str = "insert-overwrite-renamer-panel";
const ID_INSERT_OVERWRITE_METHOD_COMBO_BOX: &'static str = "insert-overwrite-method-box";
const ID_TEXT_ENTRY: &'static str = "text-entry";
const ID_AT_POSITION_SPINNER_BUTTON: &'static str = "at-position-spin-button";
const ID_AT_POSITION_COMBO_BOX: &'static str = "at-position-combo-box";

pub struct InsertOverwriteRenamer {
    builder: Builder,
    change_subject: Rc<SubjectImpl<RenamerObserverArg, Error>>,
}

impl InsertOverwriteRenamer {
    pub fn new() -> Self {
        let builder = Builder::from_string(include_str!("insert_overwrite_renamer.glade"));
        let change_subject = Rc::new(SubjectImpl::new());
        let renamer = Self {
            builder,
            change_subject,
        };

        renamer.init_callback();

        renamer
    }

    fn init_callback(&self) {
        let renamer_type = RenamerType::InsertOverwrite;
        let insert_overwrite_method_combo_box =
            self.get_object::<ComboBoxText>(ID_INSERT_OVERWRITE_METHOD_COMBO_BOX);
        let text_entry = self.get_object::<Entry>(ID_TEXT_ENTRY);
        let at_position_spin_button = self.get_object::<SpinButton>(ID_AT_POSITION_SPINNER_BUTTON);
        let at_position_combo_box = self.get_object::<ComboBoxText>(ID_AT_POSITION_COMBO_BOX);

        let change_subject = self.change_subject.clone();
        insert_overwrite_method_combo_box.connect_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        text_entry.connect_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        at_position_spin_button.connect_value_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        at_position_combo_box.connect_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });
    }

    fn get_replacement_rule(&self) -> Option<(String, InsertPosition)> {
        let insert_overwrite_method_combo_box =
            self.get_object::<ComboBoxText>(ID_INSERT_OVERWRITE_METHOD_COMBO_BOX);
        let text_entry = self.get_object::<Entry>(ID_TEXT_ENTRY);
        let at_position_spin_button = self.get_object::<SpinButton>(ID_AT_POSITION_SPINNER_BUTTON);
        let at_position_combo_box = self.get_object::<ComboBoxText>(ID_AT_POSITION_COMBO_BOX);

        let insert_overwrite_method = insert_overwrite_method_combo_box
            .get_active_id()
            .and_then(|id| match id.as_str() {
                "insert" => Some(TextInsertOrOverwrite::Insert),
                "overwrite" => Some(TextInsertOrOverwrite::Overwrite),
                _ => None,
            })
            .unwrap_or_default();
        let pos = usize::try_from(at_position_spin_button.get_value_as_int()).unwrap_or(0);
        let text_character_position =
            at_position_combo_box
                .get_active_id()
                .and_then(|id| match id.as_str() {
                    "front" => Some(TextCharPosition::Front(pos)),
                    "back" => Some(TextCharPosition::Back(pos)),
                    _ => None,
                })?;
        let insert_position = InsertPosition(text_character_position, insert_overwrite_method);

        Some((text_entry.get_text().to_string(), insert_position))
    }

    fn apply_replace_with(
        text: String,
        position: InsertPosition,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| {
                let new_file_name = match target {
                    RenamerTarget::Name => {
                        let (stem, extension) = split_file_at_dot(file_name.as_str());
                        let new_stem = position.apply_to(stem, text.as_str());
                        if let Some(suffix) = extension {
                            [new_stem.as_str(), suffix].join(".").to_string()
                        } else {
                            new_stem
                        }
                    }
                    RenamerTarget::Suffix => match split_file_at_dot(file_name.as_str()) {
                        (stem, Some(suffix)) => {
                            let new_suffix = position.apply_to(suffix, text.as_str());
                            [stem, new_suffix.as_str()].join(".").to_string()
                        }
                        (stem, None) => stem.to_string(),
                    },
                    RenamerTarget::All => position.apply_to(file_name.as_str(), text.as_str()),
                };
                (new_file_name.to_string(), dir_name.clone())
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn get_object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.get_object(name).unwrap()
    }
}

impl Renamer for InsertOverwriteRenamer {
    fn get_panel(&self) -> Container {
        self.get_object::<Container>(ID_INSERT_OVERWRITE_RENAMER_PANEL)
    }

    fn apply_replacement(
        &self,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> Result<IntoIter<(String, String)>, Error> {
        let (text, position) = self.get_replacement_rule().unwrap();
        Ok(Self::apply_replace_with(text, position, files, target))
    }

    fn attach_change(&self, observer: Rc<dyn Observer<RenamerObserverArg, Error>>) {
        self.change_subject.attach(observer);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::CounterObserver;
    use gtk::WindowBuilder;

    #[test]
    fn test_insert_overwrite_renamer_callback() {
        gtk::init().unwrap();
        let counter_observer = Rc::new(CounterObserver::new());
        let insert_overwrite_renamer = InsertOverwriteRenamer::new();
        let insert_overwrite_method_combo_box = insert_overwrite_renamer
            .get_object::<ComboBoxText>(ID_INSERT_OVERWRITE_METHOD_COMBO_BOX);
        let text_entry = insert_overwrite_renamer.get_object::<Entry>(ID_TEXT_ENTRY);
        let at_position_spin_button =
            insert_overwrite_renamer.get_object::<SpinButton>(ID_AT_POSITION_SPINNER_BUTTON);
        let at_position_combo_box =
            insert_overwrite_renamer.get_object::<ComboBoxText>(ID_AT_POSITION_COMBO_BOX);

        insert_overwrite_renamer.attach_change(counter_observer.clone());

        WindowBuilder::new()
            .child(&insert_overwrite_renamer.get_panel())
            .build()
            .show_all();

        counter_observer.reset();
        insert_overwrite_method_combo_box
            .clone()
            .set_active(Some(1));
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);

        counter_observer.reset();
        gtk_test::focus(&text_entry);
        gtk_test::enter_keys(&text_entry, "text");
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), "text".len());

        counter_observer.reset();
        gtk_test::focus(&at_position_spin_button);
        gtk_test::enter_key(&at_position_spin_button, gdk::keys::constants::uparrow);
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);

        counter_observer.reset();
        at_position_combo_box.clone().set_active(Some(1));
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);
    }

    #[test]
    fn test_insert_overwrite_renamer_apply_replacement_with() {
        assert_eq!(
            InsertOverwriteRenamer::apply_replace_with(
                "TEXT".to_string(),
                InsertPosition(TextCharPosition::Front(0), TextInsertOrOverwrite::Insert),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::All
            )
            .collect::<Vec<_>>(),
            vec![("TEXTorig.txt".to_string(), "/tmp".to_string()),]
        );

        assert_eq!(
            InsertOverwriteRenamer::apply_replace_with(
                "TEXT".to_string(),
                InsertPosition(TextCharPosition::Back(1), TextInsertOrOverwrite::Insert),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("oriTEXTg.txt".to_string(), "/tmp".to_string()),]
        );

        assert_eq!(
            InsertOverwriteRenamer::apply_replace_with(
                "TEXT".to_string(),
                InsertPosition(TextCharPosition::Front(2), TextInsertOrOverwrite::Overwrite),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::Suffix
            )
            .collect::<Vec<_>>(),
            vec![("orig.txTEXT".to_string(), "/tmp".to_string()),]
        );

        assert_eq!(
            InsertOverwriteRenamer::apply_replace_with(
                "TEXT".to_string(),
                InsertPosition(TextCharPosition::Back(3), TextInsertOrOverwrite::Overwrite),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("oTEXT.txt".to_string(), "/tmp".to_string()),]
        );
    }
}
