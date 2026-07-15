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

#[test]
fn tab_id_allocator_starts_at_one_and_never_reuses_ids() {
    let mut allocator = CompareTabIdAllocator::new();

    let first = allocator.allocate().unwrap();
    let second = allocator.allocate().unwrap();

    assert_eq!(first.get(), 1);
    assert_eq!(second.get(), 2);
    assert_eq!(allocator.last_allocated(), 2);
}

#[test]
fn tab_id_allocator_exhaustion_never_wraps() {
    let mut allocator = CompareTabIdAllocator::with_last_allocated(u64::MAX);

    assert_eq!(allocator.allocate(), Err(LoadIdentityError::TabIdExhausted));
    assert_eq!(allocator.last_allocated(), u64::MAX);
}
