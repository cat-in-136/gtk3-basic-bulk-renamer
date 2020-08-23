use crate::error::Error;
use crate::observer::{Observer, SubjectImpl};
use crate::win::provider::Renamer;
use gtk::prelude::*;
use gtk::{Builder, CheckButton, Container, Entry};
use regex::{Regex, RegexBuilder};
use std::rc::Rc;
use std::vec::IntoIter;

const ID_REPLACE_RENAMER_PANEL: &'static str = "replace-renamer-panel";
const ID_PATTERN_ENTRY: &'static str = "pattern-entry";
const ID_REGEXP_SUPPORTED: &'static str = "regexp-supported";
const ID_REPLACEMENT_ENTRY: &'static str = "replacement-entry";
const ID_CASE_SENSITIVE: &'static str = "case-sensitive";

pub struct ReplaceRenamer {
    builder: Builder,
    change_subject: Rc<SubjectImpl<(), Error>>,
}

impl ReplaceRenamer {
    pub fn new() -> Self {
        let builder = Builder::from_string(include_str!("replace_renamer.glade"));
        let change_subject = Rc::new(SubjectImpl::new());
        let renamer = Self {
            builder,
            change_subject,
        };

        renamer.init_callback();

        renamer
    }

    fn init_callback(&self) {
        let pattern_entry = self.get_object::<Entry>(ID_PATTERN_ENTRY);
        let regexp_supported = self.get_object::<CheckButton>(ID_REGEXP_SUPPORTED);
        let replacement_entry = self.get_object::<Entry>(ID_REPLACEMENT_ENTRY);
        let case_insensitive = self.get_object::<CheckButton>(ID_CASE_SENSITIVE);

        let change_subject = self.change_subject.clone();
        pattern_entry.connect_changed(move |_| {
            change_subject.notify(()).unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        regexp_supported.connect_toggled(move |_| {
            change_subject.notify(()).unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        replacement_entry.connect_changed(move |_| {
            change_subject.notify(()).unwrap_or_default();
        });

        let change_subject = self.change_subject.clone();
        case_insensitive.connect_toggled(move |_| {
            change_subject.notify(()).unwrap_or_default();
        });
    }

    fn get_replacement_rule(&self) -> Result<(Regex, String), Error> {
        let pattern = self.get_object::<Entry>(ID_PATTERN_ENTRY).get_text();
        let replacement = self.get_object::<Entry>(ID_REPLACEMENT_ENTRY).get_text();
        let is_regexp_supported = self
            .get_object::<CheckButton>(ID_REGEXP_SUPPORTED)
            .get_active();
        let is_case_sensitive = self
            .get_object::<CheckButton>(ID_CASE_SENSITIVE)
            .get_active();

        let (pattern, replacement) = if is_regexp_supported {
            (pattern.to_string(), replacement.to_string())
        } else {
            (
                regex::escape(pattern.as_str()),
                replacement.replace("$", "$$"),
            )
        };
        let matcher = RegexBuilder::new(pattern.as_str())
            .case_insensitive(!is_case_sensitive)
            .build()?;

        Ok((matcher, replacement.to_string()))
    }

    fn apply_replace_with(
        matcher: &Regex,
        replacement: &str,
        files: &[(String, String)],
    ) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| {
                let new_file_name = matcher.replace_all(file_name.as_str(), replacement);
                (new_file_name.to_string(), dir_name.clone())
            })
            .collect::<Vec<_>>()
            .into_iter()
    }

    fn get_object<T: IsA<glib::Object>>(&self, name: &str) -> T {
        self.builder.get_object(name).unwrap()
    }
}

impl Renamer for ReplaceRenamer {
    fn get_panel(&self) -> Container {
        self.get_object::<Container>(ID_REPLACE_RENAMER_PANEL)
    }

    fn apply_replacement(
        &self,
        files: &[(String, String)],
    ) -> Result<IntoIter<(String, String)>, Error> {
        let (matcher, replacement) = self.get_replacement_rule()?;
        Ok(Self::apply_replace_with(
            &matcher,
            replacement.as_str(),
            files,
        ))
    }

    fn attach_change(&self, observer: Rc<dyn Observer<(), Error>>) {
        self.change_subject.attach(observer);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use gtk::WindowBuilder;
    use std::cell::RefCell;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CounterObserver {
        count: Rc<RefCell<AtomicUsize>>,
    }

    impl CounterObserver {
        fn new() -> Self {
            Self {
                count: Rc::new(RefCell::new(AtomicUsize::new(0))),
            }
        }

        fn reset(&self) {
            let count = self.count.borrow_mut();
            count.store(0, Ordering::SeqCst);
        }

        fn count(&self) -> usize {
            let count = self.count.borrow();
            count.load(Ordering::SeqCst)
        }
    }

    impl Observer<(), Error> for CounterObserver {
        fn update(&self, arg: &()) -> Result<(), Error> {
            let count = self.count.borrow_mut();
            count.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_replace_renamer_callback() {
        gtk::init().unwrap();
        let counter_observer = Rc::new(CounterObserver::new());
        let replace_renamer = ReplaceRenamer::new();
        let pattern_entry = replace_renamer.get_object::<Entry>(ID_PATTERN_ENTRY);
        let regexp_supported = replace_renamer.get_object::<CheckButton>(ID_REGEXP_SUPPORTED);
        let replacement_entry = replace_renamer.get_object::<Entry>(ID_REPLACEMENT_ENTRY);
        let case_insensitive = replace_renamer.get_object::<CheckButton>(ID_CASE_SENSITIVE);

        replace_renamer.attach_change(counter_observer.clone());

        WindowBuilder::new()
            .child(&replace_renamer.get_panel())
            .build()
            .show_all();

        counter_observer.reset();
        gtk_test::enter_keys(&pattern_entry, "from");
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), "from".len());

        counter_observer.reset();
        gtk_test::click(&regexp_supported);
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);
        gtk_test::click(&regexp_supported);
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 2);

        counter_observer.reset();
        gtk_test::enter_keys(&replacement_entry, "to");
        assert_eq!(counter_observer.count(), "to".len());

        counter_observer.reset();
        gtk_test::click(&case_insensitive);
        assert_eq!(counter_observer.count(), 1);
        gtk_test::click(&case_insensitive);
        assert_eq!(counter_observer.count(), 2);
    }

    #[test]
    fn test_replace_renamer_apply_replacement_with() {
        let matcher = RegexBuilder::new("a+_(\\d)").build().unwrap();

        assert_eq!(
            ReplaceRenamer::apply_replace_with(
                &matcher,
                "x_$1",
                &[
                    ("a_1.txt".to_string(), "/tmp".to_string()),
                    ("aa_2_a_3.txt".to_string(), "/home/foo".to_string()),
                    ("b_1".to_string(), "/home/foo".to_string()),
                ]
            )
            .collect::<Vec<_>>(),
            vec![
                ("x_1.txt".to_string(), "/tmp".to_string()),
                ("x_2_x_3.txt".to_string(), "/home/foo".to_string()),
                ("b_1".to_string(), "/home/foo".to_string()),
            ]
        );
    }

    #[test]
    fn test_replace_renamer_get_replacement_rule_and_apply_replacement() {
        gtk::init().unwrap();
        let replace_renamer = ReplaceRenamer::new();
        let pattern_entry = replace_renamer.get_object::<Entry>(ID_PATTERN_ENTRY);
        let regexp_supported = replace_renamer.get_object::<CheckButton>(ID_REGEXP_SUPPORTED);
        let replacement_entry = replace_renamer.get_object::<Entry>(ID_REPLACEMENT_ENTRY);
        let case_insensitive = replace_renamer.get_object::<CheckButton>(ID_CASE_SENSITIVE);

        pattern_entry.set_text("a+bC(1)");
        replacement_entry.set_text("def$1");

        regexp_supported.set_active(false);
        case_insensitive.set_active(false);
        let (matcher, replacement) = replace_renamer.get_replacement_rule().unwrap();
        assert_eq!(matcher.as_str(), "a\\+bC\\(1\\)");
        assert_eq!(replacement.as_str(), "def$$1");
        assert!(matcher.is_match("A+BC(1)"));

        regexp_supported.set_active(false);
        case_insensitive.set_active(true);
        let (matcher, replacement) = replace_renamer.get_replacement_rule().unwrap();
        assert_eq!(matcher.as_str(), "a\\+bC\\(1\\)");
        assert_eq!(replacement.as_str(), "def$$1");
        assert!(!matcher.is_match("A+BC(1)"));

        regexp_supported.set_active(true);
        case_insensitive.set_active(false);
        let (matcher, replacement) = replace_renamer.get_replacement_rule().unwrap();
        assert_eq!(matcher.as_str(), "a+bC(1)");
        assert_eq!(replacement.as_str(), "def$1");
        assert!(matcher.is_match("AaBC1"));

        regexp_supported.set_active(true);
        case_insensitive.set_active(true);
        let (matcher, replacement) = replace_renamer.get_replacement_rule().unwrap();
        assert_eq!(matcher.as_str(), "a+bC(1)");
        assert_eq!(replacement.as_str(), "def$1");
        assert!(!matcher.is_match("AaBC1"));
    }
}
