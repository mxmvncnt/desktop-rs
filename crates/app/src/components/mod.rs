//! App-aware components used by two or more features (see docs/CONVENTIONS.md).

mod branch_row;
mod breadcrumbs;
mod combo_box;
mod combo_menu;
mod diff_display_options;
mod diff_stat;
mod field_row;
mod file_diff_pane;
mod file_list;
mod file_row;
mod list;
mod markdown;
mod menu_row;
mod menu_view;
mod problem_view;
mod selectable_text;
mod settings_shell;
mod suggestion_card;
mod text_field;

pub use branch_row::BranchRow;
pub use breadcrumbs::{Breadcrumbs, Crumb};
pub use combo_box::{ComboBox, ComboEntry, ComboPicked};
pub use diff_display_options::DiffDisplayOptions;
pub use diff_stat::DiffStat;
pub use field_row::FieldRow;
pub use file_list::FileList;
pub use file_row::FileRow;
pub use list::{LIST_INSET_Y, list};
pub use markdown::markdown;
pub use menu_row::{MenuRow, RowItem};
pub use menu_view::{Menu, MenuView};
pub use problem_view::ProblemView;
pub use selectable_text::{drag_selection, selectable_line};
pub use settings_shell::{NavItem, SettingsShell};
pub use suggestion_card::SuggestionCard;
pub use text_field::{text_area, text_field};
