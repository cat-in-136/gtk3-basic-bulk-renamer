use gtk::{Application, ApplicationWindow};
use std::path::PathBuf;

mod window;

pub fn create(app: &Application) -> ApplicationWindow {
    window::Window::new(app).main_window()
}

pub fn create_with_path(app: &Application, path: &[PathBuf]) -> ApplicationWindow {
    let win = window::Window::new(app);
    win.set_files(path);
    win.main_window()
}
