//! Rule engine for matching and executing rules

use crate::{Action, Condition, Error, Result, Rule, RuleId, RuleTx, TxChange};
use glam::IVec3;
use std::collections::HashMap;

/// Context for evaluating rule conditions
///
/// This trait allows the rule engine to query voxel state without
/// depending on a specific voxel implementation.
pub trait RuleContext {
    /// Get the material at a world position
    fn get_material(&self, position: IVec3) -> u8;

    /// Get the current depth at a position (for depth-based rules)
    fn get_depth(&self, position: IVec3) -> u32;

    /// Count solid neighbors (6-connected) at a position
    fn count_solid_neighbors(&self, position: IVec3) -> u8 {
        let offsets = [
            IVec3::X,
            IVec3::NEG_X,
            IVec3::Y,
            IVec3::NEG_Y,
            IVec3::Z,
            IVec3::NEG_Z,
        ];

        offsets
            .iter()
            .filter(|&&offset| self.get_material(position + offset) != 0)
            .count() as u8
    }
}

/// Executor trait for applying actions to a voxel structure
pub trait RuleExecutor {
    /// Set a voxel material at a position, returning the old material
    fn set_material(&mut self, position: IVec3, material: u8) -> u8;

    /// Get the current material at a position
    fn get_material(&self, position: IVec3) -> u8;
}

/// The rule engine manages rules and evaluates them against voxel state
#[derive(Debug, Default)]
pub struct RuleEngine {
    /// All registered rules, keyed by ID
    rules: HashMap<RuleId, Rule>,

    /// Rules sorted by priority (cached)
    sorted_rules: Vec<RuleId>,

    /// Whether the sorted cache is dirty
    cache_dirty: bool,
}

impl RuleEngine {
    /// Create a new empty rule engine
    pub fn new() -> Self {
        RuleEngine::default()
    }

    /// Add a rule to the engine
    pub fn add_rule(&mut self, rule: Rule) -> Result<()> {
        let id = rule.id().to_string();
        if self.rules.contains_key(&id) {
            return Err(Error::DuplicateRule(id));
        }
        self.rules.insert(id, rule);
        self.cache_dirty = true;
        Ok(())
    }

    /// Remove a rule from the engine
    pub fn remove_rule(&mut self, id: &str) -> Result<Rule> {
        match self.rules.remove(id) {
            Some(rule) => {
                self.cache_dirty = true;
                Ok(rule)
            }
            None => Err(Error::RuleNotFound(id.to_string())),
        }
    }

    /// Get a rule by ID
    pub fn get_rule(&self, id: &str) -> Option<&Rule> {
        self.rules.get(id)
    }

    /// Get a mutable reference to a rule by ID
    pub fn get_rule_mut(&mut self, id: &str) -> Option<&mut Rule> {
        self.rules.get_mut(id)
    }

    /// Get all rules (unsorted)
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.values()
    }

    /// Get rules sorted by priority (highest first)
    pub fn rules_by_priority(&mut self) -> impl Iterator<Item = &Rule> {
        if self.cache_dirty {
            self.rebuild_priority_cache();
        }
        self.sorted_rules.iter().filter_map(|id| self.rules.get(id))
    }

    /// Get the number of rules
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Rebuild the priority cache
    fn rebuild_priority_cache(&mut self) {
        self.sorted_rules = self.rules.keys().cloned().collect();
        self.sorted_rules.sort_by(|a, b| {
            let rule_a = self.rules.get(a).unwrap();
            let rule_b = self.rules.get(b).unwrap();
            // Sort by priority descending, then by id ascending for stability
            rule_b
                .priority()
                .cmp(&rule_a.priority())
                .then_with(|| a.cmp(b))
        });
        self.cache_dirty = false;
    }

    /// Evaluate a single condition against a context
    pub fn evaluate_condition<C: RuleContext>(
        &self,
        condition: &Condition,
        position: IVec3,
        ctx: &C,
    ) -> bool {
        match condition {
            Condition::MaterialAt { offset, material } => {
                ctx.get_material(position + *offset) == *material
            }

            Condition::EmptyAt { offset } => ctx.get_material(position + *offset) == 0,

            Condition::SolidAt { offset } => ctx.get_material(position + *offset) != 0,

            Condition::MaterialInRange { offset, min, max } => {
                let m = ctx.get_material(position + *offset);
                m >= *min && m <= *max
            }

            Condition::DepthInRange { min, max } => {
                let d = ctx.get_depth(position);
                d >= *min && d <= *max
            }

            Condition::NeighborCount { min, max } => {
                let count = ctx.count_solid_neighbors(position);
                count >= *min && count <= *max
            }

            Condition::And(conditions) => conditions
                .iter()
                .all(|c| self.evaluate_condition(c, position, ctx)),

            Condition::Or(conditions) => conditions
                .iter()
                .any(|c| self.evaluate_condition(c, position, ctx)),

            Condition::Not(condition) => !self.evaluate_condition(condition, position, ctx),

            Condition::Always => true,

            Condition::Never => false,
        }
    }

    /// Check if a rule matches at a given position
    pub fn matches<C: RuleContext>(&self, rule: &Rule, position: IVec3, ctx: &C) -> bool {
        if !rule.is_enabled() {
            return false;
        }

        // All conditions must match (empty conditions = always match)
        rule.conditions()
            .iter()
            .all(|c| self.evaluate_condition(c, position, ctx))
    }

    /// Find all matching rules at a position, in priority order
    pub fn find_matches<C: RuleContext>(&mut self, position: IVec3, ctx: &C) -> Vec<&Rule> {
        if self.cache_dirty {
            self.rebuild_priority_cache();
        }

        self.sorted_rules
            .iter()
            .filter_map(|id| self.rules.get(id))
            .filter(|rule| self.matches(rule, position, ctx))
            .collect()
    }

    /// Evaluate rules at a position and collect actions into a transaction
    pub fn evaluate<C: RuleContext>(
        &mut self,
        position: IVec3,
        ctx: &C,
        tx: &mut RuleTx,
    ) -> Vec<&Rule> {
        let matches = self.find_matches(position, ctx);

        for rule in &matches {
            for action in rule.actions() {
                tx.add_action(action.clone(), position, Some(rule.id().to_string()));
            }
        }

        matches
    }

    /// Execute a transaction against an executor
    pub fn execute<E: RuleExecutor>(&self, tx: &mut RuleTx, executor: &mut E) {
        // Collect pending actions first to avoid borrow conflict
        let actions: Vec<_> = tx
            .pending_actions()
            .map(|(a, p, r)| (a.clone(), p, r.map(String::from)))
            .collect();

        for (action, position, rule_id) in actions {
            self.execute_action(&action, position, rule_id.as_deref(), executor, tx);
        }
        tx.clear_pending();
        tx.mark_committed();
    }

    /// Execute a single action
    fn execute_action<E: RuleExecutor>(
        &self,
        action: &Action,
        position: IVec3,
        rule_id: Option<&str>,
        executor: &mut E,
        tx: &mut RuleTx,
    ) {
        match action {
            Action::SetVoxel { offset, material } => {
                let target = position + *offset;
                let old = executor.set_material(target, *material);
                tx.record_change(TxChange {
                    position: target,
                    old_material: old,
                    new_material: *material,
                    rule_id: rule_id.map(String::from),
                });
            }

            Action::ClearVoxel { offset } => {
                let target = position + *offset;
                let old = executor.set_material(target, 0);
                tx.record_change(TxChange {
                    position: target,
                    old_material: old,
                    new_material: 0,
                    rule_id: rule_id.map(String::from),
                });
            }

            Action::FillRegion { min, max, material } => {
                for x in min.x..=max.x {
                    for y in min.y..=max.y {
                        for z in min.z..=max.z {
                            let target = position + IVec3::new(x, y, z);
                            let old = executor.set_material(target, *material);
                            tx.record_change(TxChange {
                                position: target,
                                old_material: old,
                                new_material: *material,
                                rule_id: rule_id.map(String::from),
                            });
                        }
                    }
                }
            }

            Action::Replace { min, max, from, to } => {
                for x in min.x..=max.x {
                    for y in min.y..=max.y {
                        for z in min.z..=max.z {
                            let target = position + IVec3::new(x, y, z);
                            let current = executor.get_material(target);
                            if current == *from {
                                executor.set_material(target, *to);
                                tx.record_change(TxChange {
                                    position: target,
                                    old_material: *from,
                                    new_material: *to,
                                    rule_id: rule_id.map(String::from),
                                });
                            }
                        }
                    }
                }
            }

            Action::CopyRegion {
                src_min,
                src_max,
                dst_offset,
            } => {
                // First, read all source materials
                let mut source_data = Vec::new();
                for x in src_min.x..=src_max.x {
                    for y in src_min.y..=src_max.y {
                        for z in src_min.z..=src_max.z {
                            let src_pos = position + IVec3::new(x, y, z);
                            let material = executor.get_material(src_pos);
                            source_data.push((IVec3::new(x, y, z) - *src_min, material));
                        }
                    }
                }

                // Then, write to destination
                for (rel_pos, material) in source_data {
                    let dst_pos = position + *dst_offset + rel_pos;
                    let old = executor.set_material(dst_pos, material);
                    tx.record_change(TxChange {
                        position: dst_pos,
                        old_material: old,
                        new_material: material,
                        rule_id: rule_id.map(String::from),
                    });
                }
            }

            Action::Spawn { .. } | Action::Emit { .. } | Action::None => {
                // These actions don't directly modify voxels
                // They would be handled by an external system
            }
        }
    }

    /// Rollback a transaction's changes
    pub fn rollback<E: RuleExecutor>(&self, tx: &RuleTx, executor: &mut E) {
        for change in tx.rollback_changes() {
            executor.set_material(change.position, change.old_material);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Simple test context for unit tests
    struct TestContext {
        materials: HashMap<IVec3, u8>,
    }

    impl TestContext {
        fn new() -> Self {
            TestContext {
                materials: HashMap::new(),
            }
        }

        fn set(&mut self, pos: IVec3, material: u8) {
            self.materials.insert(pos, material);
        }
    }

    impl RuleContext for TestContext {
        fn get_material(&self, position: IVec3) -> u8 {
            *self.materials.get(&position).unwrap_or(&0)
        }

        fn get_depth(&self, _position: IVec3) -> u32 {
            0
        }
    }

    impl RuleExecutor for TestContext {
        fn set_material(&mut self, position: IVec3, material: u8) -> u8 {
            let old = RuleContext::get_material(self, position);
            self.materials.insert(position, material);
            old
        }

        fn get_material(&self, position: IVec3) -> u8 {
            RuleContext::get_material(self, position)
        }
    }

    // Helper function for tests to avoid method ambiguity
    fn get_ctx_material(ctx: &TestContext, pos: IVec3) -> u8 {
        RuleContext::get_material(ctx, pos)
    }

    #[test]
    fn test_engine_add_rule() {
        let mut engine = RuleEngine::new();
        let rule = Rule::new("test")
            .when(Condition::material(1))
            .then(Action::set(2));

        assert!(engine.add_rule(rule).is_ok());
        assert_eq!(engine.rule_count(), 1);

        // Duplicate should fail
        let dup = Rule::new("test");
        assert!(matches!(engine.add_rule(dup), Err(Error::DuplicateRule(_))));
    }

    #[test]
    fn test_engine_remove_rule() {
        let mut engine = RuleEngine::new();
        engine
            .add_rule(Rule::new("test").when(Condition::Always))
            .unwrap();

        assert!(engine.remove_rule("test").is_ok());
        assert_eq!(engine.rule_count(), 0);
        assert!(matches!(
            engine.remove_rule("test"),
            Err(Error::RuleNotFound(_))
        ));
    }

    #[test]
    fn test_condition_evaluation() {
        let engine = RuleEngine::new();
        let mut ctx = TestContext::new();
        ctx.set(IVec3::ZERO, 5);

        assert!(engine.evaluate_condition(&Condition::material(5), IVec3::ZERO, &ctx));
        assert!(!engine.evaluate_condition(&Condition::material(3), IVec3::ZERO, &ctx));
        assert!(engine.evaluate_condition(&Condition::solid(), IVec3::ZERO, &ctx));
        assert!(engine.evaluate_condition(&Condition::empty_at(IVec3::Y), IVec3::ZERO, &ctx));
    }

    #[test]
    fn test_rule_matching() {
        let mut engine = RuleEngine::new();
        let rule = Rule::new("grass_to_dirt")
            .when(Condition::material(2)) // GRASS
            .when(Condition::solid_at(IVec3::Y)) // Solid above
            .then(Action::set(3)); // DIRT

        engine.add_rule(rule).unwrap();

        let mut ctx = TestContext::new();
        ctx.set(IVec3::ZERO, 2); // GRASS at origin
        ctx.set(IVec3::Y, 1); // Solid above

        let rule = engine.get_rule("grass_to_dirt").unwrap();
        assert!(engine.matches(rule, IVec3::ZERO, &ctx));

        // Without solid above, shouldn't match
        ctx.set(IVec3::Y, 0);
        assert!(!engine.matches(rule, IVec3::ZERO, &ctx));
    }

    #[test]
    fn test_execute_transaction() {
        let mut engine = RuleEngine::new();
        engine
            .add_rule(
                Rule::new("test")
                    .when(Condition::material(1))
                    .then(Action::set(2)),
            )
            .unwrap();

        let mut ctx = TestContext::new();
        ctx.set(IVec3::ZERO, 1);

        let mut tx = RuleTx::new();
        engine.evaluate(IVec3::ZERO, &ctx, &mut tx);

        assert_eq!(tx.pending_count(), 1);

        engine.execute(&mut tx, &mut ctx);

        assert!(tx.is_committed());
        assert_eq!(get_ctx_material(&ctx, IVec3::ZERO), 2);
        assert_eq!(tx.changes().len(), 1);
    }

    #[test]
    fn test_rollback() {
        let engine = RuleEngine::new();
        let mut ctx = TestContext::new();
        ctx.set(IVec3::ZERO, 1);

        let mut tx = RuleTx::new();
        tx.add_action(Action::set(5), IVec3::ZERO, None);

        engine.execute(&mut tx, &mut ctx);
        assert_eq!(get_ctx_material(&ctx, IVec3::ZERO), 5);

        engine.rollback(&tx, &mut ctx);
        assert_eq!(get_ctx_material(&ctx, IVec3::ZERO), 1);
    }

    #[test]
    fn test_priority_ordering() {
        let mut engine = RuleEngine::new();

        engine
            .add_rule(Rule::new("low").with_priority(1).when(Condition::Always))
            .unwrap();
        engine
            .add_rule(Rule::new("high").with_priority(10).when(Condition::Always))
            .unwrap();
        engine
            .add_rule(Rule::new("medium").with_priority(5).when(Condition::Always))
            .unwrap();

        let rules: Vec<_> = engine.rules_by_priority().map(|r| r.id()).collect();
        assert_eq!(rules, vec!["high", "medium", "low"]);
    }
}
