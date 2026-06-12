//! Save-error dialog view-model (RFC-007, RFC-017).
//!
//! [`SaveErrorView`] maps an [`AppError`] to everything the save-error dialog
//! needs: title, body, optional path context, and an ordered list of
//! [`RecoveryButton`]s — each pairing a [`RecoveryAction`] with a concise
//! human-readable label.
//!
//! The Dioxus modal component renders `SaveErrorView` directly; no
//! match-on-error-kind logic lives in the component.
//!
//! ## Design
//!
//! `AppErrorKind::default_recovery_actions()` already orders the actions
//! correctly. This module's job is to assign a button label to each and
//! expose it as a plain struct the component can iterate.

use forskscope_core::error::{AppError, RecoveryAction};

// ── RecoveryButton ────────────────────────────────────────────────────────────

/// One button in the recovery dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryButton {
    /// Semantic action this button triggers.
    pub action: RecoveryAction,
    /// Short label shown on the button face.
    pub label:  &'static str,
    /// If `true`, render as the visually prominent primary button.
    pub is_primary: bool,
}

impl RecoveryButton {
    fn new(action: RecoveryAction, label: &'static str, is_primary: bool) -> Self {
        Self { action, label, is_primary }
    }
}

/// Human-readable button label for a [`RecoveryAction`].
///
/// Kept short enough for a dialog button — 3 words or fewer.
pub fn action_label(action: RecoveryAction) -> &'static str {
    match action {
        RecoveryAction::Dismiss            => "Dismiss",
        RecoveryAction::ChooseAnotherFile  => "Choose another file",
        RecoveryAction::Reload             => "Reload",
        RecoveryAction::SaveAs             => "Save As…",
        RecoveryAction::OverwriteAnyway    => "Overwrite anyway",
        RecoveryAction::OpenLimitedDiff    => "Open with limits",
        RecoveryAction::OpenAsBinary       => "Open as binary",
        RecoveryAction::Retry              => "Retry",
        RecoveryAction::RetryWithoutInline => "Retry without inline diff",
        RecoveryAction::Cancel             => "Cancel",
        RecoveryAction::StartFresh         => "Start fresh",
        RecoveryAction::ReportBug          => "Report bug",
    }
}

// ── SaveErrorView ─────────────────────────────────────────────────────────────

/// Everything the save-error dialog needs to render.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveErrorView {
    /// Dialog title (short, fits the title bar).
    pub title:    String,
    /// Body text — one or two sentences explaining the problem.
    pub body:     String,
    /// Optional file path shown below the body (the affected file).
    pub path:     Option<String>,
    /// Ordered recovery buttons. The first `is_primary = true` button is
    /// the safe default; destructive actions (`OverwriteAnyway`) are never
    /// primary.
    pub buttons:  Vec<RecoveryButton>,
}

impl SaveErrorView {
    /// Build a `SaveErrorView` from an [`AppError`], optionally annotating
    /// it with the file path involved.
    pub fn from_error(err: &AppError, path: Option<String>) -> Self {
        let msg     = &err.message;
        let actions = err.kind.default_recovery_actions();

        // Mark the first non-destructive action as primary.
        let buttons: Vec<RecoveryButton> = actions.iter().enumerate().map(|(i, &action)| {
            let is_primary = i == 0 && action != RecoveryAction::OverwriteAnyway
                && action != RecoveryAction::ReportBug;
            RecoveryButton::new(action, action_label(action), is_primary)
        }).collect();

        Self {
            title:   msg.short.clone(),
            body:    msg.detail.clone(),
            path,
            buttons,
        }
    }

    /// `true` when the button list contains the given action.
    pub fn has_action(&self, action: RecoveryAction) -> bool {
        self.buttons.iter().any(|b| b.action == action)
    }

    /// Return the primary button, if any.
    pub fn primary_button(&self) -> Option<&RecoveryButton> {
        self.buttons.iter().find(|b| b.is_primary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use forskscope_core::error::{AppError, AppErrorKind, RecoveryAction};

    fn err(kind: AppErrorKind) -> AppError {
        AppError::new(kind, "test detail")
    }

    // ── action_label ─────────────────────────────────────────────────────────

    #[test]
    fn all_recovery_actions_have_non_empty_labels() {
        for action in [
            RecoveryAction::Dismiss,
            RecoveryAction::ChooseAnotherFile,
            RecoveryAction::Reload,
            RecoveryAction::SaveAs,
            RecoveryAction::OverwriteAnyway,
            RecoveryAction::OpenLimitedDiff,
            RecoveryAction::OpenAsBinary,
            RecoveryAction::Retry,
            RecoveryAction::RetryWithoutInline,
            RecoveryAction::Cancel,
            RecoveryAction::StartFresh,
            RecoveryAction::ReportBug,
        ] {
            let label = action_label(action);
            assert!(!label.is_empty(), "{action:?} must have a non-empty label");
        }
    }

    #[test]
    fn dismiss_label_is_dismiss() {
        assert_eq!(action_label(RecoveryAction::Dismiss), "Dismiss");
    }

    #[test]
    fn overwrite_label_mentions_overwrite() {
        let label = action_label(RecoveryAction::OverwriteAnyway);
        assert!(label.to_lowercase().contains("overwrite"),
            "OverwriteAnyway label should mention overwrite: {label}");
    }

    // ── SaveErrorView::from_error ─────────────────────────────────────────────

    #[test]
    fn external_modification_view_has_reload_saveas_overwrite() {
        let e = err(AppErrorKind::ExternalModificationDetected);
        let v = SaveErrorView::from_error(&e, None);
        assert!(v.has_action(RecoveryAction::Reload));
        assert!(v.has_action(RecoveryAction::SaveAs));
        assert!(v.has_action(RecoveryAction::OverwriteAnyway));
    }

    #[test]
    fn external_modification_primary_button_is_reload_not_overwrite() {
        let e = err(AppErrorKind::ExternalModificationDetected);
        let v = SaveErrorView::from_error(&e, None);
        let primary = v.primary_button().expect("must have primary button");
        assert_ne!(primary.action, RecoveryAction::OverwriteAnyway,
            "OverwriteAnyway must not be primary");
        assert_eq!(primary.action, RecoveryAction::Reload);
    }

    #[test]
    fn save_conflict_view_has_correct_actions() {
        let e = err(AppErrorKind::SaveConflict);
        let v = SaveErrorView::from_error(&e, None);
        assert!(v.has_action(RecoveryAction::Reload));
        assert!(v.has_action(RecoveryAction::SaveAs));
        assert!(v.has_action(RecoveryAction::OverwriteAnyway));
    }

    #[test]
    fn write_failed_view_offers_save_as() {
        let e = err(AppErrorKind::FileWriteFailed);
        let v = SaveErrorView::from_error(&e, None);
        assert!(v.has_action(RecoveryAction::SaveAs));
        assert!(!v.has_action(RecoveryAction::OverwriteAnyway));
    }

    #[test]
    fn internal_fault_view_offers_report_bug() {
        let e = err(AppErrorKind::InternalFault);
        let v = SaveErrorView::from_error(&e, None);
        assert!(v.has_action(RecoveryAction::ReportBug));
    }

    #[test]
    fn path_is_passed_through() {
        let e = err(AppErrorKind::FileWriteFailed);
        let v = SaveErrorView::from_error(&e, Some("/home/user/file.rs".into()));
        assert_eq!(v.path.as_deref(), Some("/home/user/file.rs"));
    }

    #[test]
    fn no_path_gives_none() {
        let e = err(AppErrorKind::FileWriteFailed);
        let v = SaveErrorView::from_error(&e, None);
        assert!(v.path.is_none());
    }

    #[test]
    fn title_and_body_are_non_empty_for_save_errors() {
        for kind in [
            AppErrorKind::SaveConflict,
            AppErrorKind::ExternalModificationDetected,
            AppErrorKind::FileWriteFailed,
            AppErrorKind::BackupFailed,
        ] {
            let v = SaveErrorView::from_error(&err(kind), None);
            assert!(!v.title.is_empty(), "{kind:?} must have non-empty title");
            assert!(!v.body.is_empty(),  "{kind:?} must have non-empty body");
        }
    }

    #[test]
    fn buttons_are_non_empty_for_all_save_errors() {
        for kind in [
            AppErrorKind::SaveConflict,
            AppErrorKind::ExternalModificationDetected,
            AppErrorKind::FileWriteFailed,
            AppErrorKind::BackupFailed,
        ] {
            let v = SaveErrorView::from_error(&err(kind), None);
            assert!(!v.buttons.is_empty(), "{kind:?} must produce at least one button");
        }
    }

    #[test]
    fn each_button_has_non_empty_label() {
        let e = err(AppErrorKind::ExternalModificationDetected);
        let v = SaveErrorView::from_error(&e, None);
        for btn in &v.buttons {
            assert!(!btn.label.is_empty(), "{:?} button must have label", btn.action);
        }
    }

    #[test]
    fn exactly_one_primary_button_for_external_mod() {
        let e = err(AppErrorKind::ExternalModificationDetected);
        let v = SaveErrorView::from_error(&e, None);
        let primary_count = v.buttons.iter().filter(|b| b.is_primary).count();
        assert_eq!(primary_count, 1, "must have exactly one primary button");
    }
}
