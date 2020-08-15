use gtk::prelude::IsA;
use gtk::{Application, ApplicationWindow};
use std::path::PathBuf;

mod window;

pub fn create<P: IsA<Application>>(app: Option<&P>) -> ApplicationWindow {
    window::Window::new(app).main_window()
}

pub fn create_with_path<P: IsA<Application>>(
    app: Option<&P>,
    path: &[PathBuf],
) -> ApplicationWindow {
    let win = window::Window::new(app);
    win.set_files(path);
    win.main_window()
}
