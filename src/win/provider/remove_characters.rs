use crate::error::Error;
use crate::observer::{Observer, SubjectImpl};
use crate::utils::{split_file_at_dot, RemoveCharacterPosition, RemoveRangePosition};
use crate::win::provider::{Renamer, RenamerObserverArg, RenamerTarget, RenamerType};
use gtk::prelude::*;
use gtk::{Builder, ComboBoxText, Container, SpinButton};
use std::convert::TryFrom;
use std::rc::Rc;
use std::vec::IntoIter;

const ID_REMOVE_CHARACTERS_RENAMER_PANEL: &'static str = "remove-characters-panel";
const ID_REMOVE_FROM_SPINNER_BUTTON: &'static str = "remove-from-spin-button";
const ID_REMOVE_FROM_COMBO_BOX: &'static str = "remove-from-combo-box";
const ID_REMOVE_TO_SPINNER_BUTTON: &'static str = "remove-to-spin-button";
const ID_REMOVE_TO_COMBO_BOX: &'static str = "remove-to-combo-box";

pub struct RemoveCharactersRenamer {
    builder: Builder,
    change_subject: Rc<SubjectImpl<RenamerObserverArg, Error>>,
}

impl RemoveCharactersRenamer {
    pub fn new() -> Self {
        let builder = Builder::from_string(include_str!("remove_characters.glade"));
        let change_subject = Rc::new(SubjectImpl::new());
        let renamer = Self {
            builder,
            change_subject,
        };

        renamer.init_callback();

        renamer
    }

    fn init_callback(&self) {
        let renamer_type = RenamerType::RemoveCharacters;
        let remove_from_spin_button = self.get_object::<SpinButton>(ID_REMOVE_FROM_SPINNER_BUTTON);
        let remove_from_combo_box = self.get_object::<ComboBoxText>(ID_REMOVE_FROM_COMBO_BOX);
        let remove_to_spin_button = self.get_object::<SpinButton>(ID_REMOVE_TO_SPINNER_BUTTON);
        let remove_to_combo_box = self.get_object::<ComboBoxText>(ID_REMOVE_TO_COMBO_BOX);

        let change_subject = self.change_subject.clone();
        remove_from_spin_button.connect_value_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        remove_from_combo_box.connect_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        remove_to_spin_button.connect_value_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        remove_to_combo_box.connect_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });
    }

    fn get_replacement_rule(&self) -> Option<RemoveRangePosition> {
        let remove_from_spin_button = self.get_object::<SpinButton>(ID_REMOVE_FROM_SPINNER_BUTTON);
        let remove_from_combo_box = self.get_object::<ComboBoxText>(ID_REMOVE_FROM_COMBO_BOX);
        let remove_to_spin_button = self.get_object::<SpinButton>(ID_REMOVE_TO_SPINNER_BUTTON);
        let remove_to_combo_box = self.get_object::<ComboBoxText>(ID_REMOVE_TO_COMBO_BOX);

        let pos = usize::try_from(remove_from_spin_button.get_value_as_int()).unwrap_or(0);
        let remove_from_position = remove_from_combo_box
            .get_active_id()
            .and_then(|id| match id.as_str() {
                "front" => Some(RemoveCharacterPosition::Front(pos)),
                "back" => Some(RemoveCharacterPosition::Back(pos)),
                _ => None,
            })?;

        let pos = usize::try_from(remove_to_spin_button.get_value_as_int()).unwrap_or(0);
        let remove_to_position =
            remove_to_combo_box
                .get_active_id()
                .and_then(|id| match id.as_str() {
                    "front" => Some(RemoveCharacterPosition::Front(pos)),
                    "back" => Some(RemoveCharacterPosition::Back(pos)),
                    _ => None,
                })?;

        Some(RemoveRangePosition(
            remove_from_position,
            remove_to_position,
        ))
    }

    fn apply_replace_with(
        position: RemoveRangePosition,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| {
                let text = "";
                let new_file_name = match target {
                    RenamerTarget::Name => {
                        let (stem, extension) = split_file_at_dot(file_name.as_str());
                        let new_stem = position.apply_to(stem, text);
                        if let Some(suffix) = extension {
                            [new_stem.as_str(), suffix].join(".").to_string()
                        } else {
                            new_stem
                        }
                    }
                    RenamerTarget::Suffix => match split_file_at_dot(file_name.as_str()) {
                        (stem, Some(suffix)) => {
                            let new_suffix = position.apply_to(suffix, text);
                            [stem, new_suffix.as_str()].join(".").to_string()
                        }
                        (stem, None) => stem.to_string(),
                    },
                    RenamerTarget::All => position.apply_to(file_name.as_str(), text),
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

impl Renamer for RemoveCharactersRenamer {
    fn get_panel(&self) -> Container {
        self.get_object::<Container>(ID_REMOVE_CHARACTERS_RENAMER_PANEL)
    }

    fn apply_replacement(
        &self,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> Result<IntoIter<(String, String)>, Error> {
        let position = self.get_replacement_rule().unwrap();
        Ok(Self::apply_replace_with(position, files, target))
    }

    fn attach_change(&self, observer: Rc<dyn Observer<RenamerObserverArg, Error>>) {
        self.change_subject.attach(observer);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::observer::test::CounterObserver;
    use gtk::WindowBuilder;

    #[test]
    fn test_insert_overwrite_renamer_callback() {
        gtk::init().unwrap();
        let counter_observer = Rc::new(CounterObserver::new());
        let remove_characters_renamer = RemoveCharactersRenamer::new();
        let remove_from_spinner_button =
            remove_characters_renamer.get_object::<SpinButton>(ID_REMOVE_FROM_SPINNER_BUTTON);
        let remove_from_combo_button =
            remove_characters_renamer.get_object::<ComboBoxText>(ID_REMOVE_FROM_COMBO_BOX);
        let remove_to_spinner_button =
            remove_characters_renamer.get_object::<SpinButton>(ID_REMOVE_TO_SPINNER_BUTTON);
        let remove_to_combo_button =
            remove_characters_renamer.get_object::<ComboBoxText>(ID_REMOVE_TO_COMBO_BOX);

        remove_characters_renamer.attach_change(counter_observer.clone());

        WindowBuilder::new()
            .child(&remove_characters_renamer.get_panel())
            .build()
            .show_all();

        counter_observer.reset();
        gtk_test::focus(&remove_from_spinner_button);
        gtk_test::enter_key(&remove_from_spinner_button, gdk::keys::constants::uparrow);
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);

        counter_observer.reset();
        remove_from_combo_button.clone().set_active(Some(1));
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);

        counter_observer.reset();
        gtk_test::focus(&remove_to_spinner_button);
        gtk_test::enter_key(&remove_to_spinner_button, gdk::keys::constants::uparrow);
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);

        counter_observer.reset();
        remove_to_combo_button.clone().set_active(Some(1));
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);
    }

    #[test]
    fn test_insert_overwrite_renamer_apply_replacement_with() {
        assert_eq!(
            RemoveCharactersRenamer::apply_replace_with(
                RemoveRangePosition(
                    RemoveCharacterPosition::Front(0),
                    RemoveCharacterPosition::Front(0)
                ),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::All
            )
            .collect::<Vec<_>>(),
            vec![("orig.txt".to_string(), "/tmp".to_string()),]
        );

        assert_eq!(
            RemoveCharactersRenamer::apply_replace_with(
                RemoveRangePosition(
                    RemoveCharacterPosition::Front(1),
                    RemoveCharacterPosition::Back(1)
                ),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::All
            )
            .collect::<Vec<_>>(),
            vec![("ot".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            RemoveCharactersRenamer::apply_replace_with(
                RemoveRangePosition(
                    RemoveCharacterPosition::Back(3),
                    RemoveCharacterPosition::Front(3)
                ),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("og.txt".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            RemoveCharactersRenamer::apply_replace_with(
                RemoveRangePosition(
                    RemoveCharacterPosition::Front(1),
                    RemoveCharacterPosition::Front(2)
                ),
                &[("orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::Suffix
            )
            .collect::<Vec<_>>(),
            vec![("orig.tt".to_string(), "/tmp".to_string()),]
        );
    }
}
