use crate::error::Error;
use crate::observer::{Observer, SubjectImpl};
use crate::utils::split_file_at_dot;
use crate::win::provider::{Renamer, RenamerObserverArg, RenamerTarget, RenamerType};
use gtk::prelude::*;
use gtk::{Builder, ComboBox, Container};
use heck::*;
use std::rc::Rc;
use std::vec::IntoIter;

const ID_CHANGE_CASE_RENAMER_PANEL: &'static str = "change-case-renamer-panel";
const ID_CHANGE_CASE_COMBO_BOX: &'static str = "change-case-combo-box";

#[derive(Clone, Copy, Eq, PartialEq)]
enum ChangeCaseKind {
    Uppercase,
    Lowercase,
    CamelCase,
    SnakeCase,
    KebabCase,
    ShoutySnakeCase,
    MixedCase,
    TitleCase,
}

impl ChangeCaseKind {
    pub fn apply<T: ToString>(&self, text: T) -> String {
        match self {
            ChangeCaseKind::Uppercase => text.to_string().to_uppercase(),
            ChangeCaseKind::Lowercase => text.to_string().to_lowercase(),
            ChangeCaseKind::CamelCase => text.to_string().as_str().to_camel_case(),
            ChangeCaseKind::SnakeCase => text.to_string().as_str().to_snake_case(),
            ChangeCaseKind::KebabCase => text.to_string().as_str().to_kebab_case(),
            ChangeCaseKind::ShoutySnakeCase => text.to_string().as_str().to_shouty_snake_case(),
            ChangeCaseKind::MixedCase => text.to_string().as_str().to_mixed_case(),
            ChangeCaseKind::TitleCase => text.to_string().as_str().to_title_case(),
        }
    }
}

pub struct ChangeCaseRenamer {
    builder: Builder,
    change_subject: Rc<SubjectImpl<RenamerObserverArg, Error>>,
}

impl ChangeCaseRenamer {
    pub fn new() -> Self {
        let builder = Builder::from_string(include_str!("change_case_renamer.glade"));
        let change_subject = Rc::new(SubjectImpl::new());
        let renamer = Self {
            builder,
            change_subject,
        };

        renamer.init_callback();

        renamer
    }

    fn init_callback(&self) {
        let renamer_type = RenamerType::ChangeCase;
        let change_case_combo_box = self.get_object::<ComboBox>(ID_CHANGE_CASE_COMBO_BOX);

        let change_subject = self.change_subject.clone();
        change_case_combo_box.connect_changed(move |_| {
            change_subject
                .notify((renamer_type, ()))
                .unwrap_or_default();
        });
    }

    fn get_replacement_rule(&self) -> Option<ChangeCaseKind> {
        let change_case_combo_box = self.get_object::<ComboBox>(ID_CHANGE_CASE_COMBO_BOX);

        change_case_combo_box
            .get_active_id()
            .and_then(|id| match id.as_str() {
                "uppercase" => Some(ChangeCaseKind::Uppercase),
                "lowercase" => Some(ChangeCaseKind::Lowercase),
                "camelcase" => Some(ChangeCaseKind::CamelCase),
                "snakecase" => Some(ChangeCaseKind::SnakeCase),
                "kebabcase" => Some(ChangeCaseKind::KebabCase),
                "shoutysnakecase" => Some(ChangeCaseKind::ShoutySnakeCase),
                "mixedcase" => Some(ChangeCaseKind::MixedCase),
                "titlecase" => Some(ChangeCaseKind::TitleCase),
                _ => None,
            })
    }

    fn apply_replace_with(
        change_case_kind: ChangeCaseKind,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> IntoIter<(String, String)> {
        files
            .iter()
            .map(|(file_name, dir_name)| {
                let (stem, extension) = split_file_at_dot(file_name.as_str());

                let new_stem = match target {
                    RenamerTarget::Name | RenamerTarget::All => {
                        if stem.starts_with(".") {
                            [".".to_string(), change_case_kind.apply(&stem[1..])].concat()
                        } else {
                            change_case_kind.apply(stem)
                        }
                    }
                    RenamerTarget::Suffix => stem.to_string(),
                };
                let new_extension = extension.map(|suffix| match target {
                    RenamerTarget::Name => suffix.to_string(),
                    RenamerTarget::Suffix | RenamerTarget::All => change_case_kind.apply(suffix),
                });

                let new_file_name = if let Some(new_suffix) = new_extension {
                    [new_stem, new_suffix].join(".").to_string()
                } else {
                    new_stem
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

impl Renamer for ChangeCaseRenamer {
    fn get_panel(&self) -> Container {
        self.get_object::<Container>(ID_CHANGE_CASE_RENAMER_PANEL)
    }

    fn apply_replacement(
        &self,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> Result<IntoIter<(String, String)>, Error> {
        let change_case_kind = self.get_replacement_rule().unwrap();
        Ok(Self::apply_replace_with(change_case_kind, files, target))
    }

    fn attach_change(&self, observer: Rc<dyn Observer<(RenamerType, ()), Error>>) {
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
        let change_case_renamer = ChangeCaseRenamer::new();
        let change_case_combo_box =
            change_case_renamer.get_object::<ComboBox>(ID_CHANGE_CASE_COMBO_BOX);

        change_case_renamer.attach_change(counter_observer.clone());

        WindowBuilder::new()
            .child(&change_case_renamer.get_panel())
            .build()
            .show_all();

        counter_observer.reset();
        change_case_combo_box.clone().set_active(Some(1));
        gtk_test::wait(1);
        assert_eq!(counter_observer.count(), 1);
    }

    #[test]
    fn test_change_case_renamer_apply_replacement_with() {
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::Uppercase,
                &[("Orig.txt".to_string(), "/tmp".to_string())],
                RenamerTarget::All
            )
            .collect::<Vec<_>>(),
            vec![("ORIG.TXT".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::Lowercase,
                &[("Orig.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Suffix
            )
            .collect::<Vec<_>>(),
            vec![("Orig.txt".to_string(), "/tmp".to_string()),]
        );

        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::CamelCase,
                &[("Original file name.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("OriginalFileName.TXT".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::SnakeCase,
                &[("Original file name.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("original_file_name.TXT".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::KebabCase,
                &[("Original file name.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("original-file-name.TXT".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::ShoutySnakeCase,
                &[("Original file name.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("ORIGINAL_FILE_NAME.TXT".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::MixedCase,
                &[("Original file name.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("originalFileName.TXT".to_string(), "/tmp".to_string()),]
        );
        assert_eq!(
            ChangeCaseRenamer::apply_replace_with(
                ChangeCaseKind::TitleCase,
                &[("Original file name.TXT".to_string(), "/tmp".to_string())],
                RenamerTarget::Name
            )
            .collect::<Vec<_>>(),
            vec![("Original File Name.TXT".to_string(), "/tmp".to_string()),]
        );
    }
}
