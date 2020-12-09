use crate::error::Error;
use crate::utils::{
    split_file_at_dot, BulkTextReplacement, InsertPosition, TextCharPosition,
    TextInsertOrOverwrite, UnixTime,
};
use crate::utils::{Observer, SubjectImpl};
use crate::win::provider::{Renamer, RenamerObserverArg, RenamerTarget, RenamerType};
use gtk::prelude::*;
use gtk::{Builder, ComboBoxText, Container, Entry, SpinButton};
use std::convert::TryFrom;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::SystemTime;
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

pub struct DateTimeRenamer {
    builder: Builder,
    change_subject: Rc<SubjectImpl<RenamerObserverArg, Error>>,
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
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        format_entry.connect_changed(move |_| {
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
        let insert_position = InsertPosition(
            at_position_combo_box
                .get_active_id()
                .and_then(|id| match id.as_str() {
                    "front" => Some(TextCharPosition::Front(pos)),
                    "back" => Some(TextCharPosition::Back(pos)),
                    _ => None,
                })?,
            TextInsertOrOverwrite::Insert,
        );

        Some((
            insert_time_kind,
            format_entry.get_text().to_string(),
            insert_position,
        ))
    }

    fn get_time_for_replacement(
        insert_time_kind: InsertTimeKind,
        path: PathBuf,
    ) -> Option<UnixTime> {
        match insert_time_kind {
            InsertTimeKind::Current => Some(UnixTime::from(SystemTime::now())),
            InsertTimeKind::Accessed => path
                .metadata()
                .and_then(|metadata| metadata.accessed())
                .map(|v| UnixTime::from(v))
                .ok(),
            InsertTimeKind::Modified => path
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map(|v| UnixTime::from(v))
                .ok(),
            InsertTimeKind::PictureToken => {
                let exif = File::open(path).and_then(|file| {
                    let mut reader = BufReader::new(&file);
                    Ok(exif::Reader::new().read_from_container(&mut reader))
                });

                if let Ok(Ok(exif)) = exif {
                    exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY)
                        .or_else(|| exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY))
                        .or_else(|| exif.get_field(exif::Tag::DateTimeDigitized, exif::In::PRIMARY))
                        .and_then(|v| match v.value {
                            exif::Value::Ascii(ref vec) if !vec.is_empty() => {
                                exif::DateTime::from_ascii(&vec[0])
                                    .map(|v| UnixTime::from(v))
                                    .ok()
                            }
                            _ => None,
                        })
                } else {
                    None
                }
            }
        }
    }

    fn apply_replace_with(
        insert_time_kind: InsertTimeKind,
        pattern: String,
        position: InsertPosition,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| {
                let path = PathBuf::from(dir_name).join(file_name);
                let time = DateTimeRenamer::get_time_for_replacement(insert_time_kind, path);

                if let Some(time_str) = time.and_then(|v| v.format(pattern.as_str())) {
                    let new_file_name = match target {
                        RenamerTarget::Name => {
                            let (stem, extension) = split_file_at_dot(file_name.as_str());
                            let new_stem = position.apply_to(stem, time_str.as_str());
                            if let Some(suffix) = extension {
                                [new_stem.as_str(), suffix].join(".").to_string()
                            } else {
                                new_stem
                            }
                        }
                        RenamerTarget::Suffix => match split_file_at_dot(file_name.as_str()) {
                            (stem, Some(suffix)) => {
                                let new_suffix = position.apply_to(suffix, time_str.as_str());
                                [stem, new_suffix.as_str()].join(".").to_string()
                            }
                            (stem, None) => stem.to_string(),
                        },
                        RenamerTarget::All => {
                            position.apply_to(file_name.as_str(), time_str.as_str())
                        }
                    };
                    (new_file_name.to_string(), dir_name.clone())
                } else {
                    (file_name.to_string(), dir_name.clone())
                }
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
        target: RenamerTarget,
    ) -> Result<IntoIter<(String, String)>, Error> {
        let (insert_time_kind, pattern, position) = self.get_replacement_rule().unwrap();
        Ok(Self::apply_replace_with(
            insert_time_kind,
            pattern,
            position,
            files,
            target,
        ))
    }

    fn attach_change(&self, observer: Rc<dyn Observer<RenamerObserverArg, Error>>) {
        self.change_subject.attach(observer);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::utils::CounterObserver;
    use crate::utils::InsertPosition;
    use gtk::WindowBuilder;
    use regex::RegexBuilder;
    use std::io::{BufWriter, Write};

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
        #[rustfmt::skip]
            let sample_data: [u8; 345] = [
            0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49, 0x46, 0x00, 0x01,
            0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xff, 0xe1, 0x00, 0xb8,
            0x45, 0x78, 0x69, 0x66, 0x00, 0x00, 0x4d, 0x4d, 0x00, 0x2a, 0x00, 0x00,
            0x00, 0x08, 0x00, 0x05, 0x01, 0x1a, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x4a, 0x01, 0x1b, 0x00, 0x05, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x52, 0x01, 0x28, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x00, 0x02, 0x13, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x00, 0x87, 0x69, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x5a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x05, 0x90, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x04, 0x30, 0x32,
            0x33, 0x32, 0x90, 0x03, 0x00, 0x02, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00,
            0x00, 0x9c, 0x91, 0x01, 0x00, 0x07, 0x00, 0x00, 0x00, 0x04, 0x01, 0x02,
            0x03, 0x00, 0xa0, 0x00, 0x00, 0x07, 0x00, 0x00, 0x00, 0x04, 0x30, 0x31,
            0x30, 0x30, 0xa0, 0x01, 0x00, 0x03, 0x00, 0x00, 0x00, 0x01, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x32, 0x30, 0x30, 0x39, 0x3a, 0x30,
            0x32, 0x3a, 0x31, 0x33, 0x20, 0x32, 0x33, 0x3a, 0x33, 0x31, 0x3a, 0x33,
            0x30, 0x00, 0xff, 0xdb, 0x00, 0x43, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
            0xc0, 0x00, 0x0b, 0x08, 0x00, 0x01, 0x00, 0x01, 0x01, 0x01, 0x11, 0x00,
            0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0xff, 0xc4,
            0x00, 0x14, 0x10, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08,
            0x01, 0x01, 0x00, 0x00, 0x3f, 0x00, 0x37, 0xff, 0xd9,
        ];

        let temp_dir = tempfile::tempdir().unwrap();
        let jpg_file_path = PathBuf::from(temp_dir.path()).join("test.jpg");
        {
            let mut writer = BufWriter::new(File::create(jpg_file_path).unwrap());
            writer.write(&sample_data).unwrap();
        }
        let jpg_file_pair = (
            "test.jpg".to_string(),
            temp_dir.path().to_str().unwrap().to_string(),
        );

        let replacement = DateTimeRenamer::apply_replace_with(
            InsertTimeKind::Current,
            "%Y-%m-%d-%H-%M-%S".to_string(),
            InsertPosition(TextCharPosition::Front(1), TextInsertOrOverwrite::Insert),
            &[jpg_file_pair.clone()],
            RenamerTarget::All,
        )
        .collect::<Vec<_>>();

        assert_eq!(replacement.len(), 1);
        assert!(
            RegexBuilder::new("^t\\d{4}-\\d{2}-\\d{2}-\\d{2}-\\d{2}-\\d{2}est.jpg")
                .build()
                .unwrap()
                .is_match(replacement[0].0.as_str())
        );
        assert_eq!(jpg_file_pair.1, replacement[0].1);

        let replacement = DateTimeRenamer::apply_replace_with(
            InsertTimeKind::Accessed,
            "%Y-%m-%d-%H-%M-%S".to_string(),
            InsertPosition(TextCharPosition::Back(4), TextInsertOrOverwrite::Insert),
            &[jpg_file_pair.clone()],
            RenamerTarget::All,
        )
        .collect::<Vec<_>>();

        assert_eq!(replacement.len(), 1);
        assert!(
            RegexBuilder::new("^test\\d{4}-\\d{2}-\\d{2}-\\d{2}-\\d{2}-\\d{2}.jpg")
                .build()
                .unwrap()
                .is_match(replacement[0].0.as_str())
        );
        assert_eq!(jpg_file_pair.1, replacement[0].1);

        let replacement = DateTimeRenamer::apply_replace_with(
            InsertTimeKind::Modified,
            "%Y-%m-%d-%H-%M-%S".to_string(),
            InsertPosition(TextCharPosition::Front(0), TextInsertOrOverwrite::Insert),
            &[jpg_file_pair.clone()],
            RenamerTarget::All,
        )
        .collect::<Vec<_>>();

        assert_eq!(replacement.len(), 1);
        assert!(
            RegexBuilder::new("^\\d{4}-\\d{2}-\\d{2}-\\d{2}-\\d{2}-\\d{2}test.jpg")
                .build()
                .unwrap()
                .is_match(replacement[0].0.as_str())
        );
        assert_eq!(jpg_file_pair.1, replacement[0].1);

        let replacement = DateTimeRenamer::apply_replace_with(
            InsertTimeKind::PictureToken,
            "%Y-%m-%d-%H-%M-%S".to_string(),
            InsertPosition(TextCharPosition::Front(0), TextInsertOrOverwrite::Insert),
            &[jpg_file_pair.clone()],
            RenamerTarget::All,
        )
        .collect::<Vec<_>>();

        assert_eq!(replacement.len(), 1);
        assert!(
            RegexBuilder::new("^\\d{4}-\\d{2}-\\d{2}-\\d{2}-\\d{2}-\\d{2}test.jpg")
                .build()
                .unwrap()
                .is_match(replacement[0].0.as_str())
        );
        assert_eq!(jpg_file_pair.1, replacement[0].1);
    }
}
