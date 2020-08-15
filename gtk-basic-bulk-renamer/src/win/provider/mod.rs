use crate::win::provider::replace_renamer::ReplaceRenamer;
use gtk::Container;
use std::cell::RefCell;
use std::rc::Rc;

mod replace_renamer;

pub(crate) trait ProviderCommon {
    /// Get panel
    fn get_panel(&self) -> Container;
    /// Apply replacement
    fn apply_replacement(&self, files: &[(String, String)]) -> &[(String, String)];
}

pub struct Provider {
    replace_renamer: ReplaceRenamer,
}

impl Provider {
    pub fn new(callback: Option<Rc<RefCell<fn()>>>) -> Self {
        let replace_renamer = ReplaceRenamer::new(callback);

        Self { replace_renamer }
    }

    pub fn get_panels(&self) -> Box<[(String, Container)]> {
        vec![(
            "Search & Replace".to_string(),
            self.replace_renamer.get_panel().clone(),
        )]
        .into_boxed_slice()
    }
}
