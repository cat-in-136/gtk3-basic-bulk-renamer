use gio::prelude::ApplicationExtManual;
use gio::{ApplicationExt, ApplicationFlags, FileExt};
use gtk::{Application, WidgetExt};
use std::env;

mod basic_bulk_renamer;
mod error;
mod utils;
mod win;

fn main() {
    let application = Application::new(
        Some("io.github.cat-in-136.gtk-basic-bulk-provider"),
        ApplicationFlags::HANDLES_OPEN,
    )
    .expect("Application Initialization Error");

    application.connect_open(|application, files, _hint| {
        let path = files
            .iter()
            .filter_map(|f| f.get_path())
            .collect::<Vec<_>>();
        win::create_with_path(Some(application), &path).show_all();
    });

    application.connect_activate(|application| {
        win::create(Some(application)).show_all();
    });
    application.run(&env::args().collect::<Vec<_>>());
}
