use crate::error::Error;
use crate::utils::Observer;
use crate::win::file_list::RenamerTarget;
use crate::win::provider::change_case_renamer::ChangeCaseRenamer;
use crate::win::provider::date_time_renamer::DateTimeRenamer;
use crate::win::provider::insert_overwrite_renamer::InsertOverwriteRenamer;
use crate::win::provider::remove_characters::RemoveCharactersRenamer;
use crate::win::provider::replace_renamer::ReplaceRenamer;
use gtk::Container;
use std::rc::Rc;
use std::vec::IntoIter;
use strum_macros::{EnumIter, EnumString, IntoStaticStr};

mod change_case_renamer;
mod date_time_renamer;
mod insert_overwrite_renamer;
mod remove_characters;
mod replace_renamer;

pub(crate) trait Renamer {
    /// Get panel
    fn get_panel(&self) -> Container;
    /// Apply replacement
    fn apply_replacement(
        &self,
        files: &[(String, String)],
        target: RenamerTarget,
    ) -> Result<IntoIter<(String, String)>, Error>;
    /// Add change listener
    fn attach_change(&self, observer: Rc<dyn Observer<RenamerObserverArg, Error>>);
}

pub(crate) type RenamerObserverArg = (RenamerType, ());

#[derive(Debug, Clone, Copy, Eq, PartialEq, EnumIter, EnumString, IntoStaticStr)]
#[repr(C)]
pub(crate) enum RenamerType {
    Replace = 0,
    InsertOverwrite,
    DateTime,
    RemoveCharacters,
    ChangeCase,
}

impl RenamerType {
    pub fn label(&self) -> &'static str {
        match self {
            RenamerType::Replace => "Search & Replace",
            RenamerType::InsertOverwrite => "Insert / Overwrite",
            RenamerType::DateTime => "Insert Date/Time",
            RenamerType::RemoveCharacters => "Remove Characters",
            RenamerType::ChangeCase => "Uppercase / lowercase",
        }
    }
}

pub(crate) struct Provider {
    replace_renamer: ReplaceRenamer,
    insert_overwrite_renamer: InsertOverwriteRenamer,
    date_time_renamer: DateTimeRenamer,
    remove_characters_renamer: RemoveCharactersRenamer,
    change_case_renamer: ChangeCaseRenamer,
}

impl Provider {
    pub fn new() -> Self {
        Self {
            replace_renamer: ReplaceRenamer::new(),
            insert_overwrite_renamer: InsertOverwriteRenamer::new(),
            date_time_renamer: DateTimeRenamer::new(),
            remove_characters_renamer: RemoveCharactersRenamer::new(),
            change_case_renamer: ChangeCaseRenamer::new(),
        }
    }

    pub fn attach_change(&self, observer: Rc<dyn Observer<RenamerObserverArg, Error>>) {
        self.replace_renamer.attach_change(observer.clone());
        self.insert_overwrite_renamer
            .attach_change(observer.clone());
        self.date_time_renamer.attach_change(observer.clone());
        self.remove_characters_renamer
            .attach_change(observer.clone());
        self.change_case_renamer.attach_change(observer.clone());
    }

    pub fn renamer_of(&self, renamer_type: RenamerType) -> Box<&dyn Renamer> {
        Box::new(match renamer_type {
            RenamerType::Replace => &self.replace_renamer,
            RenamerType::InsertOverwrite => &self.insert_overwrite_renamer,
            RenamerType::DateTime => &self.date_time_renamer,
            RenamerType::RemoveCharacters => &self.remove_characters_renamer,
            RenamerType::ChangeCase => &self.change_case_renamer,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use gtk::prelude::*;
    use strum::IntoEnumIterator;

    #[test]
    fn test_provider() {
        if !gtk::is_initialized() {
            gtk::init().unwrap();
        }
        let provider = Provider::new();

        for renamer_type in RenamerType::iter() {
            let renamer = provider.renamer_of(renamer_type);
            let label = renamer_type.label();
            let panel = renamer.get_panel();

            assert!(label.len() > 0);
            assert!(panel.children().len() > 0);
        }
    }
}
