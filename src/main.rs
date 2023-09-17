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

#[cfg(test)]
mod test {

    pub fn test_synced<F, R>(function: F) -> R
    where
        F: FnOnce() -> R + Send + std::panic::UnwindSafe + 'static,
        R: Send + 'static,
    {
        // skip_assert_initialized!();

        use std::panic;
        use std::sync::mpsc;

        let (tx, rx) = mpsc::sync_channel(1);
        TEST_THREAD_WORKER
            .push(move || {
                tx.send(panic::catch_unwind(function))
                    .unwrap_or_else(|_| panic!("Failed to return result from thread pool"));
            })
            .expect("Failed to schedule a test call");
        rx.recv()
            .expect("Failed to receive result from thread pool")
            .unwrap_or_else(|e| std::panic::resume_unwind(e))
    }

    static TEST_THREAD_WORKER: glib::once_cell::sync::Lazy<glib::ThreadPool> =
        glib::once_cell::sync::Lazy::new(|| {
            let pool = glib::ThreadPool::exclusive(1).unwrap();
            pool.push(move || {
                gtk::init().expect("Tests failed to initialize gtk");
            })
            .expect("Failed to schedule a test call");
            pool
        });
}
