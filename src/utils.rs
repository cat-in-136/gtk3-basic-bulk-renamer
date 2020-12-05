use glib::{filename_from_uri, DateTime, TimeZone, Value};
use gtk::prelude::*;
use gtk::{ListStore, SelectionData};
use std::iter;
use std::path::PathBuf;
use std::time::SystemTime;

pub fn value2string(value: &Value) -> String {
    value
        .get::<String>()
        .unwrap_or(None)
        .unwrap_or_default()
        .clone()
}

pub fn list_store_data_iter(model: &ListStore) -> impl Iterator<Item = Vec<Value>> + '_ {
    let n_column = model.get_n_columns();

    let mut current_iter = model.get_iter_first();
    iter::repeat_with(move || {
        if let Some(iter) = &current_iter {
            let value = (0..n_column)
                .map(|column| model.get_value(&iter, column as i32))
                .collect::<Vec<_>>();
            if !model.iter_next(&iter) {
                current_iter = None
            }
            Some(value)
        } else {
            None
        }
    })
    .take_while(|v| v.is_some())
    .map(|v| v.unwrap())
}

pub(crate) fn get_path_from_selection_data(sel_data: &SelectionData) -> Vec<PathBuf> {
    if sel_data.targets_include_uri() {
        sel_data
            .get_uris()
            .iter()
            .filter_map(|v| {
                filename_from_uri(v.as_str())
                    .and_then(|(path, _hostname)| Ok(path))
                    .ok()
            })
            .collect::<Vec<_>>()
    } else if let Some(text) = sel_data.get_text() {
        text.to_string()
            .lines()
            .filter_map(|v| {
                if v.starts_with("file:") {
                    filename_from_uri(v)
                        .and_then(|(path, _hostname)| Ok(path))
                        .ok()
                } else {
                    Some(PathBuf::from(v))
                }
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    }
}

pub(crate) fn split_file_at_dot(file: &str) -> (&str, Option<&str>) {
    if file == "." || file == ".." {
        (file, None)
    } else {
        let mut iter = file.rsplitn(2, ".");
        let after = iter.next();
        let before = iter.next();
        match (before, after) {
            (None, None) => ("", None),
            (Some(""), _) => (file, None),
            (None, Some(_)) => (file, None),
            (Some(before), after) => (before, after),
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum InsertPosition {
    Front(usize),
    Back(usize),
    FrontOverwrite(usize),
    BackOverwrite(usize),
}

impl InsertPosition {
    pub fn apply_to(self, text: &str, replacement: &str) -> String {
        let idx = match self {
            InsertPosition::Front(pos) | InsertPosition::FrontOverwrite(pos) => pos,
            InsertPosition::Back(pos) | InsertPosition::BackOverwrite(pos) => {
                text.len().checked_sub(pos).unwrap_or(0)
            }
        }
        .min(text.len());

        let mut new_text = text.to_string();
        match self {
            InsertPosition::Front(_) | InsertPosition::Back(_) => {
                new_text.insert_str(idx, &replacement);
            }
            InsertPosition::FrontOverwrite(_) | InsertPosition::BackOverwrite(_) => {
                let range = idx..(idx + replacement.len()).min(text.len());
                new_text.replace_range(range, &replacement);
            }
        }
        new_text
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) struct UnixTime(pub i64);

impl UnixTime {
    pub fn to_glib_date_time(&self) -> DateTime {
        DateTime::from_unix_local(self.0)
    }
    pub fn format(&self, format: &str) -> Option<String> {
        self.to_glib_date_time()
            .format(format)
            .map(|v| v.to_string())
    }
}

impl From<SystemTime> for UnixTime {
    fn from(time: SystemTime) -> Self {
        Self(if time > SystemTime::UNIX_EPOCH {
            time.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        } else {
            -(SystemTime::UNIX_EPOCH
                .duration_since(time)
                .unwrap()
                .as_secs() as i64)
        })
    }
}

impl From<DateTime> for UnixTime {
    fn from(datetime: DateTime) -> Self {
        Self(datetime.to_unix())
    }
}

impl From<exif::DateTime> for UnixTime {
    fn from(datetime: exif::DateTime) -> Self {
        Self::from(DateTime::new(
            &TimeZone::new(
                datetime
                    .offset
                    .map(|offset| {
                        format!(
                            "{}{:02}:{:02}",
                            if offset >= 0 { '+' } else { '-' },
                            offset.abs() / 60,
                            offset.abs() % 60
                        )
                    })
                    .as_deref(),
            ),
            datetime.year as i32,
            datetime.month as i32,
            datetime.day as i32,
            datetime.hour as i32,
            datetime.minute as i32,
            datetime.second as f64 + (datetime.nanosecond.unwrap_or_default() as f64 / 1000000.0),
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use glib::bitflags::_core::time::Duration;
    use glib::Type;
    use gtk::{Clipboard, GtkListStoreExt, ListStore};
    use regex::RegexBuilder;

    #[test]
    fn test_value2string() {
        let val = Value::from(Some("value"));
        assert_eq!(value2string(&val), "value".to_string());
    }

    #[test]
    fn test_list_store_data_iter() {
        gtk::init().unwrap();
        let list_store = ListStore::new(&[Type::String, Type::String]);
        assert_eq!(
            list_store_data_iter(&list_store).collect::<Vec<_>>().len(),
            0
        );

        for i in 0..3 {
            let iter = list_store.append();
            list_store.set(
                &iter,
                &[0, 1],
                &[&format!("{}", i).to_string(), &"dummy".to_string()],
            );
        }
        assert_eq!(
            list_store_data_iter(&list_store)
                .map(|v| v.iter().map(|v| v.get::<String>()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![
                vec![Ok(Some("0".to_string())), Ok(Some("dummy".to_string()))],
                vec![Ok(Some("1".to_string())), Ok(Some("dummy".to_string()))],
                vec![Ok(Some("2".to_string())), Ok(Some("dummy".to_string()))],
            ]
        );
    }

    #[test]
    fn test_get_path_from_selection_data() {
        gtk::init().unwrap();

        let clipboard = Clipboard::get(&gdk::SELECTION_CLIPBOARD);
        clipboard.clear();
        clipboard.set_text("file:///tmp/test\n/home/test/foobar");

        let selection = clipboard.wait_for_contents(&gdk::TARGET_STRING).unwrap();
        assert_eq!(
            get_path_from_selection_data(&selection),
            vec![
                PathBuf::from("/tmp/test"),
                PathBuf::from("/home/test/foobar")
            ]
        );
    }

    #[test]
    fn test_split_file_at_dot() {
        assert_eq!(split_file_at_dot(""), ("", None));
        assert_eq!(split_file_at_dot("."), (".", None));
        assert_eq!(split_file_at_dot(".."), ("..", None));
        assert_eq!(split_file_at_dot(".hidden"), (".hidden", None));
        assert_eq!(split_file_at_dot(".hidden.txt"), (".hidden", Some("txt")));
        assert_eq!(split_file_at_dot("file_name"), ("file_name", None));
        assert_eq!(
            split_file_at_dot("file_name.txt"),
            ("file_name", Some("txt"))
        );
        assert_eq!(
            split_file_at_dot("file.name.txt"),
            ("file.name", Some("txt"))
        );
    }

    #[test]
    fn test_insert_position() {
        use InsertPosition::*;

        assert_eq!(Front(0).apply_to("text", "INS"), "INStext");
        assert_eq!(Front(1).apply_to("text", "INS"), "tINSext");
        assert_eq!(Front(3).apply_to("text", "INS"), "texINSt");
        assert_eq!(Front(4).apply_to("text", "INS"), "textINS");
        assert_eq!(Front(5).apply_to("text", "INS"), "textINS");

        assert_eq!(Back(0).apply_to("text", "INS"), "textINS");
        assert_eq!(Back(1).apply_to("text", "INS"), "texINSt");
        assert_eq!(Back(3).apply_to("text", "INS"), "tINSext");
        assert_eq!(Back(4).apply_to("text", "INS"), "INStext");
        assert_eq!(Back(5).apply_to("text", "INS"), "INStext");

        assert_eq!(FrontOverwrite(0).apply_to("text", "OW"), "OWxt");
        assert_eq!(FrontOverwrite(1).apply_to("text", "OW"), "tOWt");
        assert_eq!(FrontOverwrite(2).apply_to("text", "OW"), "teOW");
        assert_eq!(FrontOverwrite(3).apply_to("text", "OW"), "texOW");
        assert_eq!(FrontOverwrite(4).apply_to("text", "OW"), "textOW");
        assert_eq!(FrontOverwrite(5).apply_to("text", "OW"), "textOW");

        assert_eq!(BackOverwrite(0).apply_to("text", "OW"), "textOW");
        assert_eq!(BackOverwrite(1).apply_to("text", "OW"), "texOW");
        assert_eq!(BackOverwrite(2).apply_to("text", "OW"), "teOW");
        assert_eq!(BackOverwrite(3).apply_to("text", "OW"), "tOWt");
        assert_eq!(BackOverwrite(4).apply_to("text", "OW"), "OWxt");
        assert_eq!(BackOverwrite(5).apply_to("text", "OW"), "OWxt");
    }

    #[test]
    fn test_unix_time() {
        let matcher = RegexBuilder::new("^\\d{4}-\\d{2}-\\d{2}-%-\\d{2}:\\d{2}:\\d{2}$")
            .build()
            .unwrap();

        let time = UnixTime::from(SystemTime::now());
        let text = time.format("%Y-%m-%d-%%-%H:%M:%S").unwrap();
        assert!(matcher.is_match(text.as_str()));

        let time = UnixTime::from(SystemTime::UNIX_EPOCH);
        let text = time.format("%Y-%m-%d-%%-%H:%M:%S").unwrap();
        assert!(matcher.is_match(text.as_str()));

        let time = UnixTime::from(
            SystemTime::UNIX_EPOCH
                .checked_sub(Duration::from_secs(1))
                .unwrap(),
        );
        let text = time.format("%Y-%m-%d-%%-%H:%M:%S").unwrap();
        assert!(matcher.is_match(text.as_str()));
    }
}
