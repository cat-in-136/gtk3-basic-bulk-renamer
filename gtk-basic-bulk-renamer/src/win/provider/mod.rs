use crate::error::Error;
use crate::observer::Observer;
use crate::win::file_list::RenamerTarget;
use crate::win::provider::date_time_renamer::DateTimeRenamer;
use crate::win::provider::replace_renamer::ReplaceRenamer;
use gtk::Container;
use std::rc::Rc;
use std::vec::IntoIter;
use strum_macros::EnumIter;

mod date_time_renamer;
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

#[derive(Debug, Clone, Copy, Eq, PartialEq, EnumIter)]
#[repr(C)]
pub(crate) enum RenamerType {
    Replace = 0,
    DateTime = 1,
}

impl RenamerType {
    pub fn label(&self) -> &'static str {
        match self {
            RenamerType::Replace => "Search & Replace",
            RenamerType::DateTime => "Insert Date/Time",
        }
    }
}

pub(crate) struct Provider {
    replace_renamer: ReplaceRenamer,
    date_time_renamer: DateTimeRenamer,
}

impl Provider {
    pub fn new() -> Self {
        Self {
            replace_renamer: ReplaceRenamer::new(),
            date_time_renamer: DateTimeRenamer::new(),
        }
    }

    pub fn attach_change(&self, observer: Rc<dyn Observer<RenamerObserverArg, Error>>) {
        self.replace_renamer.attach_change(observer.clone());
        self.date_time_renamer.attach_change(observer.clone());
    }

    pub fn renamer_of(&self, renamer_type: RenamerType) -> Box<&dyn Renamer> {
        Box::new(match renamer_type {
            RenamerType::Replace => &self.replace_renamer,
            RenamerType::DateTime => &self.date_time_renamer,
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
        gtk::init().unwrap();
        let provider = Provider::new();

        for renamer_type in RenamerType::iter() {
            let renamer = provider.renamer_of(renamer_type);
            let label = renamer_type.label();
            let panel = renamer.get_panel();

            assert!(label.len() > 0);
            assert!(panel.get_children().len() > 0);
        }
    }
}
