use crate::domain::category::CategoryMember;
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use std::sync::Arc;

/// Takes a deleted source out of the categories holding it.
#[derive(Clone)]
pub struct CategoryMemberRemover {
    repository: Arc<dyn CategoryRepository>,
}

impl CategoryMemberRemover {
    pub fn new(repository: Arc<dyn CategoryRepository>) -> Self {
        Self { repository }
    }
}

pub trait CategoryMemberRemoverApi: Send + Sync {
    /// Removes `member` from every category holding it; categories stay.
    fn remove_from_all(&self, member: &CategoryMember) -> anyhow::Result<()>;
}

impl CategoryMemberRemoverApi for CategoryMemberRemover {
    fn remove_from_all(&self, member: &CategoryMember) -> anyhow::Result<()> {
        self.repository
            .list()?
            .into_iter()
            .filter(|category| category.has_member(member))
            .map(|category| category.without_member(member))
            .try_for_each(|category| self.repository.update(&category))
    }
}
