use glib::{filename_from_uri, Value};
use gtk::prelude::*;
use gtk::{ListStore, SelectionData};
use std::iter;
use std::path::PathBuf;

mod datetime;
mod insert_position;
mod observer;
pub(crate) use datetime::*;
pub(crate) use insert_position::*;
#[cfg(test)]
pub(crate) use observer::test::CounterObserver;
pub(crate) use observer::*;

pub fn value2string(value: &Value) -> String {
    value.get::<String>().unwrap_or_default().clone()
}

pub fn list_store_data_iter(model: &ListStore) -> impl Iterator<Item = Vec<Value>> + '_ {
    let n_column = model.n_columns();

    let mut current_iter = model.iter_first();
    iter::repeat_with(move || {
        if let Some(iter) = &current_iter {
            let value = (0..n_column)
                .map(|column| model.value(&iter, column as i32))
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
            .uris()
            .iter()
            .filter_map(|v| {
                filename_from_uri(v.as_str())
                    .and_then(|(path, _hostname)| Ok(path))
                    .ok()
            })
            .collect::<Vec<_>>()
    } else if let Some(text) = sel_data.text() {
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::test::test_synced;
    use glib::Type;
    use gtk::Clipboard;

    #[test]
    fn test_value2string() {
        let val = Value::from(Some("value"));
        assert_eq!(value2string(&val), "value".to_string());
    }

    #[test]
    fn test_list_store_data_iter() {
        test_synced(move || {
            let list_store = ListStore::new(&[Type::STRING, Type::STRING]);
            assert_eq!(
                list_store_data_iter(&list_store).collect::<Vec<_>>().len(),
                0
            );

            for i in 0..3 {
                let iter = list_store.append();
                list_store.set(
                    &iter,
                    &[
                        (0, &format!("{}", i).to_string()),
                        (1, &"dummy".to_string()),
                    ],
                );
            }
            assert_eq!(
                list_store_data_iter(&list_store)
                    .map(|v| v
                        .iter()
                        .map(|val| val.get::<String>())
                        .map(|val| val.ok())
                        .collect::<Vec<_>>())
                    .collect::<Vec<_>>()
                    .as_slice(),
                &[
                    vec![Some("0".to_string()), Some("dummy".to_string())],
                    vec![Some("1".to_string()), Some("dummy".to_string())],
                    vec![Some("2".to_string()), Some("dummy".to_string())],
                ]
            );
        });
    }

    #[test]
    fn test_get_path_from_selection_data() {
        test_synced(move || {
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
        });
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
}
