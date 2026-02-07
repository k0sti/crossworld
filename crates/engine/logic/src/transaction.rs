//! Transaction system for batching and tracking rule changes

use crate::Action;
use glam::IVec3;
use serde::{Deserialize, Serialize};

/// A recorded change from a rule action
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxChange {
    /// World position where the change occurred
    pub position: IVec3,

    /// Previous material value (for rollback)
    pub old_material: u8,

    /// New material value
    pub new_material: u8,

    /// Rule ID that caused this change (if any)
    pub rule_id: Option<String>,
}

/// Transaction for batching rule actions
///
/// A `RuleTx` collects actions and changes, allowing for atomic commit
/// or rollback of multiple operations.
#[derive(Debug, Default)]
pub struct RuleTx {
    /// Pending actions to execute
    pending_actions: Vec<PendingAction>,

    /// Recorded changes (after execution)
    changes: Vec<TxChange>,

    /// Whether the transaction has been committed
    committed: bool,
}

/// An action pending execution with its context
#[derive(Debug, Clone)]
struct PendingAction {
    /// The action to perform
    action: Action,

    /// Base position for the action
    position: IVec3,

    /// Rule ID that generated this action
    rule_id: Option<String>,
}

impl RuleTx {
    /// Create a new empty transaction
    pub fn new() -> Self {
        RuleTx::default()
    }

    /// Check if the transaction has any pending actions
    pub fn is_empty(&self) -> bool {
        self.pending_actions.is_empty()
    }

    /// Get the number of pending actions
    pub fn pending_count(&self) -> usize {
        self.pending_actions.len()
    }

    /// Get the recorded changes
    pub fn changes(&self) -> &[TxChange] {
        &self.changes
    }

    /// Check if the transaction has been committed
    pub fn is_committed(&self) -> bool {
        self.committed
    }

    /// Add an action to the transaction
    pub fn add_action(&mut self, action: Action, position: IVec3, rule_id: Option<String>) {
        self.pending_actions.push(PendingAction {
            action,
            position,
            rule_id,
        });
    }

    /// Add a change record (typically called during execution)
    pub fn record_change(&mut self, change: TxChange) {
        self.changes.push(change);
    }

    /// Get pending actions for processing
    pub fn pending_actions(&self) -> impl Iterator<Item = (&Action, IVec3, Option<&str>)> {
        self.pending_actions
            .iter()
            .map(|pa| (&pa.action, pa.position, pa.rule_id.as_deref()))
    }

    /// Mark the transaction as committed
    pub fn mark_committed(&mut self) {
        self.committed = true;
    }

    /// Clear all pending actions (used after commit or rollback)
    pub fn clear_pending(&mut self) {
        self.pending_actions.clear();
    }

    /// Get rollback actions (returns changes in reverse order)
    pub fn rollback_changes(&self) -> impl Iterator<Item = &TxChange> {
        self.changes.iter().rev()
    }

    /// Merge another transaction into this one
    pub fn merge(&mut self, other: RuleTx) {
        self.pending_actions.extend(other.pending_actions);
        self.changes.extend(other.changes);
    }

    /// Take ownership of changes and return them
    pub fn take_changes(self) -> Vec<TxChange> {
        self.changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation() {
        let tx = RuleTx::new();
        assert!(tx.is_empty());
        assert!(!tx.is_committed());
    }

    #[test]
    fn test_transaction_add_action() {
        let mut tx = RuleTx::new();
        tx.add_action(
            Action::set(5),
            IVec3::new(1, 2, 3),
            Some("test_rule".into()),
        );

        assert_eq!(tx.pending_count(), 1);
        assert!(!tx.is_empty());
    }

    #[test]
    fn test_transaction_record_change() {
        let mut tx = RuleTx::new();
        tx.record_change(TxChange {
            position: IVec3::new(1, 2, 3),
            old_material: 0,
            new_material: 5,
            rule_id: Some("test_rule".into()),
        });

        assert_eq!(tx.changes().len(), 1);
        assert_eq!(tx.changes()[0].new_material, 5);
    }

    #[test]
    fn test_transaction_merge() {
        let mut tx1 = RuleTx::new();
        tx1.add_action(Action::set(1), IVec3::ZERO, None);

        let mut tx2 = RuleTx::new();
        tx2.add_action(Action::set(2), IVec3::ONE, None);

        tx1.merge(tx2);
        assert_eq!(tx1.pending_count(), 2);
    }
}
