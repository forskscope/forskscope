//! Runtime identity for asynchronous compare loads (RFC-075).
//!
//! A background completion is allowed to mutate a tab only when its immutable
//! [`CompareTabId`] and per-tab [`LoadGeneration`] still match a live loading
//! tab. Vector position is deliberately absent from this model.
//!
//! These identities are process-local concurrency tokens. They are unrelated
//! to legacy persisted workspace IDs and are never restored from disk.

use std::fmt;

/// Failure to allocate or advance a runtime load identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadIdentityError {
    /// Zero is reserved for invalid or uninitialized tab identities.
    InvalidTabId,
    /// Zero is reserved for invalid or uninitialized load generations.
    InvalidGeneration,
    /// Advancing the generation would wrap and reuse an old token.
    GenerationExhausted,
}

impl fmt::Display for LoadIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTabId => f.write_str("compare tab ID must be non-zero"),
            Self::InvalidGeneration => f.write_str("load generation must be non-zero"),
            Self::GenerationExhausted => f.write_str("load generation is exhausted"),
        }
    }
}

impl std::error::Error for LoadIdentityError {}

/// Unique identity of a compare tab for one process lifetime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CompareTabId(u64);

impl CompareTabId {
    /// Create an ID allocated by the owning store.
    pub const fn new(value: u64) -> Result<Self, LoadIdentityError> {
        if value == 0 {
            Err(LoadIdentityError::InvalidTabId)
        } else {
            Ok(Self(value))
        }
    }

    /// Numeric value for diagnostics and deterministic tests.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Monotonically increasing identity of one load attempt within a tab.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LoadGeneration(u64);

impl LoadGeneration {
    /// Generation assigned to a newly opened compare tab.
    pub const INITIAL: Self = Self(1);

    /// Construct a generation from validated runtime state.
    pub const fn new(value: u64) -> Result<Self, LoadIdentityError> {
        if value == 0 {
            Err(LoadIdentityError::InvalidGeneration)
        } else {
            Ok(Self(value))
        }
    }

    /// Return the next generation without ever wrapping to a reused value.
    pub const fn next(self) -> Result<Self, LoadIdentityError> {
        match self.0.checked_add(1) {
            Some(value) => Ok(Self(value)),
            None => Err(LoadIdentityError::GenerationExhausted),
        }
    }

    /// Numeric value for diagnostics and deterministic tests.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Complete identity captured by one asynchronous compare load.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LoadToken {
    pub tab_id: CompareTabId,
    pub generation: LoadGeneration,
}

impl LoadToken {
    pub const fn new(tab_id: CompareTabId, generation: LoadGeneration) -> Self {
        Self { tab_id, generation }
    }
}

/// Live identity/state projected from a candidate tab during completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadIdentitySnapshot {
    pub token: LoadToken,
    pub is_loading: bool,
}

impl LoadIdentitySnapshot {
    pub const fn new(token: LoadToken, is_loading: bool) -> Self {
        Self { token, is_loading }
    }
}

/// Outcome of validating a background completion against current tab state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionDecision {
    Accept,
    RejectTabMissing,
    RejectGenerationMismatch,
    RejectNotLoading,
}

/// Decide whether `expected` may commit to the current candidate tab.
///
/// Callers normally locate the candidate by `expected.tab_id` first and pass
/// `None` when it no longer exists. Rechecking the candidate ID here keeps the
/// integrity rule self-contained and safe against an incorrect lookup.
pub fn completion_decision(
    expected: LoadToken,
    current: Option<LoadIdentitySnapshot>,
) -> CompletionDecision {
    let Some(current) = current else {
        return CompletionDecision::RejectTabMissing;
    };

    if current.token.tab_id != expected.tab_id {
        return CompletionDecision::RejectTabMissing;
    }
    if current.token.generation != expected.generation {
        return CompletionDecision::RejectGenerationMismatch;
    }
    if !current.is_loading {
        return CompletionDecision::RejectNotLoading;
    }
    CompletionDecision::Accept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: u64) -> CompareTabId {
        CompareTabId::new(value).unwrap()
    }

    fn generation(value: u64) -> LoadGeneration {
        LoadGeneration::new(value).unwrap()
    }

    fn token(tab_id: u64, generation_value: u64) -> LoadToken {
        LoadToken::new(id(tab_id), generation(generation_value))
    }

    #[test]
    fn same_token_while_loading_is_accepted() {
        let expected = token(7, 3);
        let current = LoadIdentitySnapshot::new(expected, true);

        assert_eq!(
            completion_decision(expected, Some(current)),
            CompletionDecision::Accept
        );
    }

    #[test]
    fn absent_tab_is_rejected() {
        assert_eq!(
            completion_decision(token(7, 3), None),
            CompletionDecision::RejectTabMissing
        );
    }

    #[test]
    fn different_tab_is_rejected_even_at_same_generation() {
        let expected = token(7, 3);
        let other = LoadIdentitySnapshot::new(token(8, 3), true);

        assert_eq!(
            completion_decision(expected, Some(other)),
            CompletionDecision::RejectTabMissing
        );
    }

    #[test]
    fn older_generation_is_rejected() {
        let expected = token(7, 2);
        let current = LoadIdentitySnapshot::new(token(7, 3), true);

        assert_eq!(
            completion_decision(expected, Some(current)),
            CompletionDecision::RejectGenerationMismatch
        );
    }

    #[test]
    fn unexpected_newer_generation_is_also_rejected() {
        let expected = token(7, 4);
        let current = LoadIdentitySnapshot::new(token(7, 3), true);

        assert_eq!(
            completion_decision(expected, Some(current)),
            CompletionDecision::RejectGenerationMismatch
        );
    }

    #[test]
    fn matching_token_outside_loading_state_is_rejected() {
        let expected = token(7, 3);
        let current = LoadIdentitySnapshot::new(expected, false);

        assert_eq!(
            completion_decision(expected, Some(current)),
            CompletionDecision::RejectNotLoading
        );
    }

    #[test]
    fn generation_mismatch_takes_precedence_over_state() {
        let expected = token(7, 2);
        let current = LoadIdentitySnapshot::new(token(7, 3), false);

        assert_eq!(
            completion_decision(expected, Some(current)),
            CompletionDecision::RejectGenerationMismatch
        );
    }

    #[test]
    fn zero_values_are_reserved() {
        assert_eq!(CompareTabId::new(0), Err(LoadIdentityError::InvalidTabId));
        assert_eq!(
            LoadGeneration::new(0),
            Err(LoadIdentityError::InvalidGeneration)
        );
    }

    #[test]
    fn initial_generation_is_one_and_advances_monotonically() {
        let first = LoadGeneration::INITIAL;
        let second = first.next().unwrap();
        let third = second.next().unwrap();

        assert_eq!(first.get(), 1);
        assert_eq!(second.get(), 2);
        assert_eq!(third.get(), 3);
        assert!(first < second && second < third);
    }

    #[test]
    fn generation_exhaustion_never_wraps() {
        let maximum = LoadGeneration::new(u64::MAX).unwrap();

        assert_eq!(maximum.next(), Err(LoadIdentityError::GenerationExhausted));
    }

    #[test]
    fn token_and_id_values_are_available_for_diagnostics() {
        let token = token(42, 9);

        assert_eq!(token.tab_id.get(), 42);
        assert_eq!(token.generation.get(), 9);
    }
}
