use crate::error::Error;
use crate::utils::{list_store_data_iter, value2string};
use crate::win::provider::Renamer;
use basic_bulk_renamer::RenameMapPair;
use gtk::prelude::*;
use gtk::ListStore;
use std::path::PathBuf;

pub(super) fn set_files_to_file_list(file_list_store: &ListStore, paths: &[PathBuf]) {
    file_list_store.clear();
    add_files_to_file_list(&file_list_store, paths);
}

pub(super) fn add_files_to_file_list(file_list_store: &ListStore, paths: &[PathBuf]) {
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

pub(super) fn get_files_from_file_list(
    file_list_store: &ListStore,
) -> impl Iterator<Item = RenameMapPair> + '_ {
    list_store_data_iter(file_list_store).map(|v| {
        let name = value2string(&v[0]);
        let new_name = value2string(&v[1]);
        let parent = value2string(&v[2]);

        let parent_name = PathBuf::from(parent);
        let file_name = parent_name.join(name);
        let new_file_name = parent_name.join(new_name);

        (file_name, new_file_name)
    })
}

pub(super) fn apply_renamer_to_file_list(
    file_list_store: &ListStore,
    renamer: Box<&dyn Renamer>,
) -> Result<(), Error> {
    let data = list_store_data_iter(&file_list_store)
        .map(|row| (value2string(&row[0]), value2string(&row[2])))
        .collect::<Vec<_>>();

    if let Some(iter) = file_list_store.get_iter_first() {
        renamer
            .apply_replacement(data.as_slice())
            .and_then(|replacements| {
                for (new_file_name, _) in replacements {
                    file_list_store.set(&iter, &[1], &[&new_file_name]);
                    file_list_store.iter_next(&iter);
                }
                Ok(())
            })
    } else {
        Ok(()) // TODO
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::observer::Observer;
    use glib::Type;
    use gtk::Container;
    use std::rc::Rc;
    use std::vec::IntoIter;

    fn list_store() -> ListStore {
        ListStore::new(&[Type::String, Type::String, Type::String])
    }

    struct TestRenamer {
        prefix: String,
    }

    impl TestRenamer {
        fn into_boxed_dyn(&self) -> Box<&dyn Renamer> {
            Box::new(self)
        }
    }

    impl Renamer for TestRenamer {
        fn get_panel(&self) -> Container {
            unimplemented!()
        }

        fn apply_replacement(
            &self,
            files: &[(String, String)],
        ) -> Result<IntoIter<(String, String)>, Error> {
            Ok(files
                .iter()
                .map(|(name, parent)| {
                    (
                        [self.prefix.clone(), name.to_string()].join("-"),
                        parent.clone(),
                    )
                })
                .collect::<Vec<_>>()
                .into_iter())
        }

        fn attach_change(&self, _observer: Rc<dyn Observer<(), Error>>) {
            unimplemented!()
        }
    }

    #[test]
    fn test_add_files_to_file_list() {
        gtk::init().unwrap();

        let file_list_store = list_store();
        assert_eq!(file_list_store.iter_n_children(None), 0);

        add_files_to_file_list(
            &file_list_store,
            &[PathBuf::from("test"), PathBuf::from("/test2")],
        );
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

    #[test]
    fn test_get_files_from_file_list() {
        gtk::init().unwrap();

        let file_list_store = list_store();

        assert_eq!(
            get_files_from_file_list(&file_list_store).collect::<Vec<_>>(),
            vec![]
        );

        let iter = file_list_store.append();
        file_list_store.set(
            &iter,
            &[0, 1, 2],
            &[&"test".to_string(), &"test2".to_string(), &"/".to_string()],
        );

        assert_eq!(
            get_files_from_file_list(&file_list_store).collect::<Vec<_>>(),
            vec![(
                PathBuf::from("/").join("test"),
                PathBuf::from("/").join("test2")
            )]
        );

        let iter = file_list_store.append();
        file_list_store.set(
            &iter,
            &[0, 1, 2],
            &[
                &"test3".to_string(),
                &"test4".to_string(),
                &"/tmp".to_string(),
            ],
        );

        assert_eq!(
            get_files_from_file_list(&file_list_store).collect::<Vec<_>>(),
            vec![
                (
                    PathBuf::from("/").join("test"),
                    PathBuf::from("/").join("test2")
                ),
                (
                    PathBuf::from("/tmp").join("test3"),
                    PathBuf::from("/tmp").join("test4")
                ),
            ]
        );
    }

    #[test]
    fn test_apply_renamer_to_file_list() {
        gtk::init().unwrap();

        let file_list_store = list_store();
        let test_renamer = TestRenamer {
            prefix: "ABC".to_string(),
        };
        let test_renamer = test_renamer.into_boxed_dyn();

        apply_renamer_to_file_list(&file_list_store, test_renamer.clone()).unwrap();

        let iter = file_list_store.append();
        file_list_store.set(
            &iter,
            &[0, 1, 2],
            &[&"test".to_string(), &"test2".to_string(), &"/".to_string()],
        );

        apply_renamer_to_file_list(&file_list_store, test_renamer.clone()).unwrap();

        let iter = file_list_store.iter_nth_child(None, 0).unwrap();
        assert_eq!(
            file_list_store.get_value(&iter, 0).get(),
            Ok(Some(String::from("test")))
        );
        assert_eq!(
            file_list_store.get_value(&iter, 1).get(),
            Ok(Some(String::from("ABC-test")))
        );
        assert_eq!(
            file_list_store.get_value(&iter, 2).get(),
            Ok(Some(String::from("/")))
        );
    }
}
