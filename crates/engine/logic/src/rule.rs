//! Rule definition and management

use crate::{Action, Condition};
use serde::{Deserialize, Serialize};

/// Unique identifier for a rule
pub type RuleId = String;

/// A rule that matches conditions and dispatches actions
///
/// Rules are evaluated in priority order. When a rule's conditions match,
/// its actions are collected for execution. Rules with the same priority
/// are evaluated in insertion order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier for this rule
    id: RuleId,

    /// Human-readable description
    description: Option<String>,

    /// Conditions that must all match (AND logic)
    conditions: Vec<Condition>,

    /// Actions to perform when conditions match
    actions: Vec<Action>,

    /// Priority (higher = evaluated first)
    priority: i32,

    /// Whether this rule is currently enabled
    enabled: bool,

    /// Tags for categorization and filtering
    tags: Vec<String>,
}

impl Rule {
    /// Create a new rule with the given ID
    pub fn new(id: impl Into<RuleId>) -> Self {
        Rule {
            id: id.into(),
            description: None,
            conditions: Vec::new(),
            actions: Vec::new(),
            priority: 0,
            enabled: true,
            tags: Vec::new(),
        }
    }

    /// Get the rule's unique identifier
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get the rule's description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Get the rule's conditions
    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    /// Get the rule's actions
    pub fn actions(&self) -> &[Action] {
        &self.actions
    }

    /// Get the rule's priority
    pub fn priority(&self) -> i32 {
        self.priority
    }

    /// Check if the rule is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get the rule's tags
    pub fn tags(&self) -> &[String] {
        &self.tags
    }

    /// Set the rule's description (builder pattern)
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a condition to this rule (builder pattern)
    pub fn when(mut self, condition: Condition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Add multiple conditions to this rule (builder pattern)
    pub fn when_all(mut self, conditions: impl IntoIterator<Item = Condition>) -> Self {
        self.conditions.extend(conditions);
        self
    }

    /// Add an action to this rule (builder pattern)
    pub fn then(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    /// Add multiple actions to this rule (builder pattern)
    pub fn then_all(mut self, actions: impl IntoIterator<Item = Action>) -> Self {
        self.actions.extend(actions);
        self
    }

    /// Set the rule's priority (builder pattern)
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set whether the rule is enabled (builder pattern)
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Add a tag to this rule (builder pattern)
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags to this rule (builder pattern)
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(Into::into));
        self
    }

    /// Enable or disable this rule
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if this rule has any conditions
    pub fn has_conditions(&self) -> bool {
        !self.conditions.is_empty()
    }

    /// Check if this rule has any actions
    pub fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    /// Check if this rule has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

impl PartialEq for Rule {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Rule {}

impl std::hash::Hash for Rule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::IVec3;

    #[test]
    fn test_rule_builder() {
        let rule = Rule::new("test_rule")
            .with_description("A test rule")
            .with_priority(10)
            .when(Condition::material(1))
            .when(Condition::empty_at(IVec3::Y))
            .then(Action::set(2))
            .with_tag("test");

        assert_eq!(rule.id(), "test_rule");
        assert_eq!(rule.description(), Some("A test rule"));
        assert_eq!(rule.priority(), 10);
        assert_eq!(rule.conditions().len(), 2);
        assert_eq!(rule.actions().len(), 1);
        assert!(rule.has_tag("test"));
        assert!(rule.is_enabled());
    }

    #[test]
    fn test_rule_serialization() {
        let rule = Rule::new("serialize_test")
            .when(Condition::material(5))
            .then(Action::set(10));

        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: Rule = serde_json::from_str(&json).unwrap();

        assert_eq!(rule.id(), deserialized.id());
        assert_eq!(rule.conditions().len(), deserialized.conditions().len());
    }
}
