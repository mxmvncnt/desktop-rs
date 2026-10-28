//! App-aware components used by two or more features (see docs/CONVENTIONS.md).

mod branch_row;
mod diff_display_options;
mod diff_stat;
mod file_diff_pane;
mod file_list;
mod file_row;
mod list;
mod menu_row;
mod menu_view;
mod suggestion_card;
mod text_field;

pub use branch_row::BranchRow;
pub use diff_display_options::DiffDisplayOptions;
pub use diff_stat::DiffStat;
pub use file_list::FileList;
pub use file_row::FileRow;
pub use list::{LIST_INSET_Y, list};
pub use menu_row::{MenuRow, RowItem};
pub use menu_view::{Menu, MenuView};
pub use suggestion_card::SuggestionCard;
pub use text_field::{text_area, text_field};
