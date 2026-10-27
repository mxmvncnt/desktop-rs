//! App-aware components used by two or more features (see docs/CONVENTIONS.md).

mod diff_display_options;
mod file_list;
mod file_row;

pub use diff_display_options::DiffDisplayOptions;
pub use file_list::{FileList, FileSource};
pub use file_row::FileRow;
