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
    /// Allocating another tab ID would wrap and reuse an old identity.
    TabIdExhausted,
}

impl fmt::Display for LoadIdentityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTabId => f.write_str("compare tab ID must be non-zero"),
            Self::InvalidGeneration => f.write_str("load generation must be non-zero"),
            Self::GenerationExhausted => f.write_str("load generation is exhausted"),
            Self::TabIdExhausted => f.write_str("compare tab ID space is exhausted"),
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

/// Process-local, never-reusing source of compare-tab identities.
///
/// Store this allocator at the application root. Closing a tab does not return
/// its ID to the allocator, and exhaustion fails closed instead of wrapping.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CompareTabIdAllocator {
    last_allocated: u64,
}

impl CompareTabIdAllocator {
    /// Construct an allocator whose next ID is 1.
    pub const fn new() -> Self {
        Self { last_allocated: 0 }
    }

    /// Allocate the next process-local identity without wrapping or reuse.
    pub fn allocate(&mut self) -> Result<CompareTabId, LoadIdentityError> {
        let value = self
            .last_allocated
            .checked_add(1)
            .ok_or(LoadIdentityError::TabIdExhausted)?;
        self.last_allocated = value;
        Ok(CompareTabId(value))
    }

    /// Numeric high-water mark for diagnostics and deterministic tests.
    pub const fn last_allocated(self) -> u64 {
        self.last_allocated
    }

    #[cfg(test)]
    const fn with_last_allocated(last_allocated: u64) -> Self {
        Self { last_allocated }
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
mod tests;
