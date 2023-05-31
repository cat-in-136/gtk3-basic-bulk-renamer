use gio::prelude::*;
use gio::prelude::{ApplicationExt, ApplicationExtManual};
use gio::ApplicationFlags;
use gtk::prelude::*;
use gtk::Application;

mod basic_bulk_renamer;
mod error;
mod utils;
mod win;

fn main() {
    let application = Application::new(
        Some("io.github.cat-in-136.gtk-basic-bulk-provider"),
        ApplicationFlags::HANDLES_OPEN,
    );

    application.connect_open(|application, files, _hint| {
        let path = files.iter().filter_map(|f| f.path()).collect::<Vec<_>>();
        win::create_with_path(Some(application), path.as_slice()).show_all();
    });

    application.connect_activate(|application| {
        win::create(Some(application)).show_all();
    });
    application.run();
}
