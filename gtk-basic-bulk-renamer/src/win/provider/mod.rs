use crate::error::Error;
use crate::observer::Observer;
use crate::win::provider::replace_renamer::ReplaceRenamer;
use gtk::Container;
use std::rc::Rc;
use std::vec::IntoIter;
use strum_macros::EnumIter;

mod replace_renamer;

pub(crate) trait Renamer {
    /// Get panel
    fn get_panel(&self) -> Container;
    /// Apply replacement
    fn apply_replacement(
        &self,
        files: &[(String, String)],
    ) -> Result<IntoIter<(String, String)>, Error>;
    /// Add change listener
    fn attach_change(&self, observer: Rc<dyn Observer<(), Error>>);
}

#[derive(Debug, Clone, Copy, EnumIter)]
#[repr(C)]
pub(crate) enum RenamerType {
    Replace = 0,
}

impl RenamerType {
    pub fn label(&self) -> &'static str {
        match self {
            RenamerType::Replace => "Search & Replace",
        }
    }
}

pub(crate) struct Provider {
    replace_renamer: ReplaceRenamer,
}

impl Provider {
    pub fn new() -> Self {
        let replace_renamer = ReplaceRenamer::new();

        Self { replace_renamer }
    }

    pub fn renamer_of(&self, renamer_type: RenamerType) -> Box<&dyn Renamer> {
        Box::new(match renamer_type {
            RenamerType::Replace => &self.replace_renamer,
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
