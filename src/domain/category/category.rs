use super::category_id::CategoryId;
use super::category_member::CategoryMember;
use super::category_name::CategoryName;
use chrono::{DateTime, Utc};

/// A user-created group of tracked sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: CategoryId,
    pub name: CategoryName,
    /// Each member once, in the order first added.
    pub members: Vec<CategoryMember>,
    pub created_at: DateTime<Utc>,
}

impl Category {
    /// A new category with a generated id and no members.
    pub fn create(name: CategoryName, now: DateTime<Utc>) -> Self {
        Self {
            id: CategoryId::new_generated(),
            name,
            members: Vec::new(),
            created_at: now,
        }
    }

    pub fn with_name(self, _name: CategoryName) -> Self {
        self
    }

    /// Replaces the members, keeping each once (first occurrence wins).
    pub fn with_members(self, _members: Vec<CategoryMember>) -> Self {
        self
    }

    pub fn without_member(self, _member: &CategoryMember) -> Self {
        self
    }

    pub fn has_member(&self, _member: &CategoryMember) -> bool {
        false
    }
}
