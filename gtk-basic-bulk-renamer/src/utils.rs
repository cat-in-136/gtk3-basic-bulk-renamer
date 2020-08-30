use glib::{filename_from_uri, Value};
use gtk::prelude::*;
use gtk::{ListStore, SelectionData};
use std::iter;
use std::path::PathBuf;

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

#[cfg(test)]
mod test {
    use super::*;
    use glib::Type;
    use gtk::{Clipboard, GtkListStoreExt, ListStore};

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
}
