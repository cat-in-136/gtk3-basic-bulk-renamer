# gtk3-basic-bulk-renamer

This project is currently under beta stage of development.

gtk3-basic-bulk-renamer is a GTK+3-based bulk renamer utility to change the name of multiple files at once.

![Rust](https://github.com/cat-in-136/gtk3-basic-bulk-renamer/workflows/Rust/badge.svg)

## Install

DEB package and RPM package is available on the release page.

## Build

Dependencies:

 * Rust 1.42+
 * GTK+3

To build, just run `cargo build`. To run, just run `cargo run`.

# Usage

GUI design is almost same as Thunar Bulk Rename unitity.

 1. Add the files to be renamed
    * To add files,
      * Click "+" button to open file dialog; or
        * Drop file from another applications
      * To remove files,
        * Select file and click "-" button
 2. Choose renaming target
    * "Name": the name of the files;
    * "Suffix": the suffix of the files; or
    * "All": entire file name i.e. both the name and the suffix of the files
 3. Choose renaming rule from the tab
    * Search & Replace
    * Insert / Overwrite
    * Insert Date/Time
    * Remove Characters
    * Uppercase / lowercase
 4. Enter option of renaming rule
    * As you enter the value, a preview of the changes will be displayed in the "New Name" column on the table.
 5. Click "Rename" button

Tips: For cinnamon/Nemo users, to use this application from Nemo, enter gtk3-basic-bulk-renamer in Edit > Preferences > Behaviour > Bulk Rename

## License

MIT License. See the LICENSE.txt file.
