//! Text editing operation model tests (RFC-032 §"Operation Rules",
//! §"Transaction Model", §"Core Types").

use crate::edit_op::{
    DocumentId, EditTransaction, OperationReject, RejectReason, RevisionId,
    TextEditOperation, TextOffset, TextRange, TransactionId, TransactionLabel,
    is_revision_compatible,
};

// ── RevisionId ────────────────────────────────────────────────────────────────

#[test]
fn revision_id_initial_is_zero() {
    assert_eq!(RevisionId::initial().0, 0);
    assert!(RevisionId::initial().is_initial());
}

#[test]
fn revision_id_next_increments() {
    let r = RevisionId(3);
    assert_eq!(r.next(), RevisionId(4));
}

#[test]
fn revision_id_ordering_is_ascending() {
    assert!(RevisionId(1) < RevisionId(2));
    assert!(RevisionId(0) < RevisionId(100));
}

// ── TextRange ─────────────────────────────────────────────────────────────────

#[test]
fn text_range_len_is_end_minus_start() {
    assert_eq!(TextRange::new(3, 7).len(), 4);
    assert_eq!(TextRange::new(0, 0).len(), 0);
}

#[test]
fn text_range_is_empty_when_start_equals_end() {
    assert!(TextRange::empty_at(5).is_empty());
    assert!(!TextRange::new(2, 5).is_empty());
}

#[test]
fn text_range_contains_start_but_not_end() {
    let r = TextRange::new(3, 7);
    assert!( r.contains(TextOffset(3)));
    assert!( r.contains(TextOffset(6)));
    assert!(!r.contains(TextOffset(7)), "end is exclusive");
    assert!(!r.contains(TextOffset(2)));
}

#[test]
fn text_range_overlaps_when_they_intersect() {
    let a = TextRange::new(0, 5);
    let b = TextRange::new(3, 8);
    assert!(a.overlaps(b));
    assert!(b.overlaps(a), "overlap must be symmetric");
}

#[test]
fn text_range_does_not_overlap_adjacent_ranges() {
    let a = TextRange::new(0, 3);
    let b = TextRange::new(3, 6); // start of b == end of a
    assert!(!a.overlaps(b), "adjacent ranges must not overlap");
}

// ── TextEditOperation ─────────────────────────────────────────────────────────

fn doc() -> DocumentId { DocumentId::new("doc-1") }
fn rev(n: u64) -> RevisionId { RevisionId(n) }

#[test]
fn insert_op_document_id_and_base_revision() {
    let op = TextEditOperation::Insert {
        document:      doc(),
        base_revision: rev(5),
        offset:        TextOffset(10),
        text:          "hello".into(),
    };
    assert_eq!(op.document_id(), &doc());
    assert_eq!(op.base_revision(), rev(5));
}

#[test]
fn delete_op_affected_range_matches_range() {
    let range = TextRange::new(4, 9);
    let op = TextEditOperation::Delete {
        document:      doc(),
        base_revision: rev(2),
        range,
    };
    assert_eq!(op.affected_range(), range);
}

#[test]
fn replace_op_affected_range_matches_range() {
    let range = TextRange::new(1, 6);
    let op = TextEditOperation::Replace {
        document:      doc(),
        base_revision: rev(0),
        range,
        text:          "world".into(),
    };
    assert_eq!(op.affected_range(), range);
}

#[test]
fn insert_op_affected_range_is_empty_at_offset() {
    let op = TextEditOperation::Insert {
        document:      doc(),
        base_revision: rev(1),
        offset:        TextOffset(7),
        text:          "x".into(),
    };
    assert!(op.affected_range().is_empty());
    assert_eq!(op.affected_range().start.0, 7);
}

#[test]
fn insert_inserts_text_returns_true() {
    let op = TextEditOperation::Insert {
        document: doc(), base_revision: rev(0),
        offset: TextOffset(0), text: "a".into(),
    };
    assert!(op.inserts_text());
    assert!(!op.deletes_text());
}

#[test]
fn delete_deletes_text_returns_true() {
    let op = TextEditOperation::Delete {
        document: doc(), base_revision: rev(0),
        range: TextRange::new(0, 3),
    };
    assert!(op.deletes_text());
    assert!(!op.inserts_text());
}

#[test]
fn replace_both_inserts_and_deletes() {
    let op = TextEditOperation::Replace {
        document: doc(), base_revision: rev(0),
        range: TextRange::new(0, 5), text: "new".into(),
    };
    assert!(op.inserts_text());
    assert!(op.deletes_text());
}

#[test]
fn replace_with_empty_text_only_deletes() {
    let op = TextEditOperation::Replace {
        document: doc(), base_revision: rev(0),
        range: TextRange::new(0, 3), text: String::new(),
    };
    assert!(!op.inserts_text());
    assert!(op.deletes_text());
}

// ── Revision compatibility ────────────────────────────────────────────────────

#[test]
fn same_revision_is_compatible() {
    assert!(is_revision_compatible(rev(7), rev(7)));
}

#[test]
fn stale_revision_is_not_compatible() {
    // RFC-032 rule 2: operation is rejected when base_revision ≠ current.
    assert!(!is_revision_compatible(rev(3), rev(7)));
    assert!(!is_revision_compatible(rev(8), rev(7)));
}

// ── OperationReject ───────────────────────────────────────────────────────────

#[test]
fn stale_revision_reject_has_correct_reason() {
    let reject = OperationReject {
        document:           doc(),
        submitted_revision: rev(3),
        current_revision:   rev(5),
        reason:             RejectReason::StaleRevision,
    };
    assert_eq!(reject.reason, RejectReason::StaleRevision);
    assert_eq!(reject.current_revision, rev(5));
}

// ── TransactionLabel ─────────────────────────────────────────────────────────

#[test]
fn well_known_transaction_labels_are_non_empty() {
    for label in [
        TransactionLabel::merge_hunk_left_to_right(),
        TransactionLabel::merge_hunk_right_to_left(),
        TransactionLabel::manual_edit(),
        TransactionLabel::paste(),
        TransactionLabel::delete_selection(),
    ] {
        assert!(!label.as_str().is_empty(), "label must be non-empty");
    }
}

// ── EditTransaction ───────────────────────────────────────────────────────────

#[test]
fn empty_transaction_is_empty_and_not_reversible_without_inverse() {
    let tx = EditTransaction::new(
        TransactionId::new("tx-1"),
        TransactionLabel::manual_edit(),
        vec![],
        vec![],
    );
    assert!(tx.is_empty());
    assert!(!tx.is_reversible());
}

#[test]
fn transaction_with_operations_and_inverse_is_reversible() {
    let op = TextEditOperation::Insert {
        document: doc(), base_revision: rev(0),
        offset: TextOffset(0), text: "hello".into(),
    };
    let inverse = TextEditOperation::Delete {
        document: doc(), base_revision: rev(1),
        range: TextRange::new(0, 5),
    };
    let tx = EditTransaction::new(
        TransactionId::new("tx-2"),
        TransactionLabel::manual_edit(),
        vec![op],
        vec![inverse],
    );
    assert!(!tx.is_empty());
    assert!(tx.is_reversible());
}

#[test]
fn transaction_id_equality() {
    let a = TransactionId::new("abc");
    let b = TransactionId::new("abc");
    let c = TransactionId::new("xyz");
    assert_eq!(a, b);
    assert_ne!(a, c);
}
