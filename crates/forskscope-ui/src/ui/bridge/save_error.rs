//! Re-exports the save-error dialog view-model from `forskscope-ui-logic`
//! (RFC-020 §5a, RFC-007, RFC-017).
//! `SaveErrorView::from_error(err, path)` maps an `AppError` to the
//! title, body, and ordered `Vec<RecoveryButton>` the error dialog renders.
#[allow(unused_imports)]
pub use forskscope_ui_logic::{RecoveryButton, SaveErrorView, action_label};
