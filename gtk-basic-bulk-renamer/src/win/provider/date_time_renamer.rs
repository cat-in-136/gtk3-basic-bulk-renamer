use crate::error::Error;
use crate::observer::{Observer, SubjectImpl};
use crate::win::provider::{Renamer, RenamerType};
use chrono::Local;
use gtk::prelude::*;
use gtk::{Builder, ComboBoxText, Container, Entry, SpinButton};
use std::convert::TryFrom;
use std::rc::Rc;
use std::vec::IntoIter;

const ID_DATE_TIME_RENAMER_PANEL: &'static str = "date-time-renamer-panel";
const ID_INSERT_TIME_COMBO_BOX: &'static str = "insert-time-combo-box";
const ID_FORMAT_ENTRY: &'static str = "format-entry";
const ID_AT_POSITION_SPINNER_BUTTON: &'static str = "at-position-spin-button";
const ID_AT_POSITION_COMBO_BOX: &'static str = "at-position-combo-box";

#[derive(Clone, Copy, Eq, PartialEq)]
enum InsertTimeKind {
    Current,
    Accessed,
    Modified,
    PictureToken,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum InsertPosition {
    Front(usize),
    Back(usize),
}

pub struct DateTimeRenamer {
    builder: Builder,
    change_subject: Rc<SubjectImpl<(RenamerType), Error>>,
}

impl DateTimeRenamer {
    pub fn new() -> Self {
        let builder = Builder::from_string(include_str!("date_time_renamer.glade"));
        let change_subject = Rc::new(SubjectImpl::new());
        let renamer = Self {
            builder,
            change_subject,
        };

        renamer.init_callback();

        renamer
    }

    fn init_callback(&self) {
        let renamer_type = RenamerType::DateTime;
        let insert_time_combo_box = self.get_object::<ComboBoxText>(ID_INSERT_TIME_COMBO_BOX);
        let format_entry = self.get_object::<Entry>(ID_FORMAT_ENTRY);
        let at_position_spin_button = self.get_object::<SpinButton>(ID_AT_POSITION_SPINNER_BUTTON);
        let at_position_combo_box = self.get_object::<ComboBoxText>(ID_AT_POSITION_COMBO_BOX);

        let change_subject = self.change_subject.clone();
        insert_time_combo_box.connect_changed(move |_| {
            change_subject.notify((renamer_type)).unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        format_entry.connect_changed(move |_| {
            change_subject.notify((renamer_type)).unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        at_position_spin_button.connect_change_value(move |_, _| {
            change_subject.notify((renamer_type)).unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        at_position_combo_box.connect_changed(move |_| {
            change_subject.notify((renamer_type)).unwrap_or_default();
        });
    }

    fn get_replacement_rule(&self) -> Option<(InsertTimeKind, String, InsertPosition)> {
        let insert_time_combo_box = self.get_object::<ComboBoxText>(ID_INSERT_TIME_COMBO_BOX);
        let format_entry = self.get_object::<Entry>(ID_FORMAT_ENTRY);
        let at_position_spin_button = self.get_object::<SpinButton>(ID_AT_POSITION_SPINNER_BUTTON);
        let at_position_combo_box = self.get_object::<ComboBoxText>(ID_AT_POSITION_COMBO_BOX);

        let insert_time_kind =
            insert_time_combo_box
                .get_active_id()
                .and_then(|id| match id.as_str() {
                    "current" => Some(InsertTimeKind::Current),
                    "accessed" => Some(InsertTimeKind::Accessed),
                    "modified" => Some(InsertTimeKind::Modified),
                    "picture-taken" => Some(InsertTimeKind::PictureToken),
                    _ => None,
                })?;
        let pos = usize::try_from(at_position_spin_button.get_value_as_int()).unwrap_or(0);
        let insert_position =
            at_position_combo_box
                .get_active_id()
                .and_then(|id| match id.as_str() {
                    "front" => Some(InsertPosition::Front(pos)),
                    "back" => Some(InsertPosition::Back(pos)),
                    _ => None,
                })?;

        Some((
            insert_time_kind,
            format_entry.get_text().to_string(),
            insert_position,
        ))
    }

    fn apply_replace_with(
        insert_time_kind: InsertTimeKind,
        pattern: String,
        position: InsertPosition,
        files: &[(String, String)],
    ) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| {
                let time = Local::now(); // TODO
                let time = time.format(pattern.as_str()).to_string();

                let mut new_file_name = file_name.clone();
                match position {
                    InsertPosition::Front(pos) => new_file_name.insert_str(pos, &time),
                    InsertPosition::Back(pos) => {
                        new_file_name.insert_str(file_name.len() - pos, &time)
                    }
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

impl Renamer for DateTimeRenamer {
    fn get_panel(&self) -> Container {
        self.get_object::<Container>(ID_DATE_TIME_RENAMER_PANEL)
    }

    fn apply_replacement(
        &self,
        files: &[(String, String)],
    ) -> Result<IntoIter<(String, String)>, Error> {
        let (insert_time_kind, pattern, position) = self.get_replacement_rule().unwrap();
        Ok(Self::apply_replace_with(
            insert_time_kind,
            pattern,
            position,
            files,
        ))
    }

    fn attach_change(&self, observer: Rc<dyn Observer<(RenamerType), Error>>) {
        self.change_subject.attach(observer);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::observer::test::CounterObserver;
    use gtk::{Adjustment, WindowBuilder};
    use regex::RegexBuilder;

    #[test]
    fn test_replace_renamer_callback() {
        gtk::init().unwrap();
        let counter_observer = Rc::new(CounterObserver::new());
        let date_time_renamer = DateTimeRenamer::new();
        let insert_time_combo_box =
            date_time_renamer.get_object::<ComboBoxText>(ID_INSERT_TIME_COMBO_BOX);
        let format_entry = date_time_renamer.get_object::<Entry>(ID_FORMAT_ENTRY);
        let at_position_spin_button =
            date_time_renamer.get_object::<SpinButton>(ID_AT_POSITION_SPINNER_BUTTON);
        let at_position_combo_box =
            date_time_renamer.get_object::<ComboBoxText>(ID_AT_POSITION_COMBO_BOX);

        date_time_renamer.attach_change(counter_observer.clone());

        WindowBuilder::new()
            .child(&date_time_renamer.get_panel())
            .build()
            .show_all();

        counter_observer.reset();
        insert_time_combo_box.clone().set_active(Some(1));
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);

        counter_observer.reset();
        gtk_test::focus(&format_entry);
        gtk_test::enter_keys(&format_entry, "%Y-%d");
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), "%Y-%d".len());

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
    fn test_apply_replace_with() {
        let replacement = DateTimeRenamer::apply_replace_with(
            InsertTimeKind::Current,
            "%Y-%m-%d-%H-%M-%S".to_string(),
            InsertPosition::Front(1),
            &[("foobar".to_string(), "/tmp".to_string())],
        )
        .collect::<Vec<_>>();

        assert_eq!(replacement.len(), 1);
        assert!(
            RegexBuilder::new("^f\\d{4}-\\d{2}-\\d{2}-\\d{2}-\\d{2}-\\d{2}oobar$")
                .build()
                .unwrap()
                .is_match(replacement[0].0.as_str())
        );
        assert_eq!("/tmp", replacement[0].1.as_str());
    }
}
