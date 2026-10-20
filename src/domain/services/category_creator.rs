use crate::domain::category::{Category, CategoryName, CreateCategoryError};
use crate::infrastructure::repositories::sqlite_category_repository::CategoryRepository;
use crate::infrastructure::shared::system_clock::Clock;
use std::sync::Arc;

/// Creates categories, each with a name no other category has.
#[derive(Clone)]
pub struct CategoryCreator {
    repository: Arc<dyn CategoryRepository>,
    clock: Arc<dyn Clock>,
}

impl CategoryCreator {
    pub fn new(repository: Arc<dyn CategoryRepository>, clock: Arc<dyn Clock>) -> Self {
        Self { repository, clock }
    }
}

pub trait CategoryCreatorApi: Send + Sync {
    /// Creates a category with no members. Another category with the same
    /// name ignoring case → `NameTaken`.
    fn create(&self, name: CategoryName) -> Result<Category, CreateCategoryError>;
}

impl CategoryCreatorApi for CategoryCreator {
    fn create(&self, name: CategoryName) -> Result<Category, CreateCategoryError> {
        self.ensure_name_free(&name)?;
        let category = Category::create(name, self.clock.now());
        self.repository
            .insert(&category)
            .map_err(CreateCategoryError::Repository)?;
        Ok(category)
    }
}

impl CategoryCreator {
    fn ensure_name_free(&self, name: &CategoryName) -> Result<(), CreateCategoryError> {
        let taken = self
            .repository
            .list()
            .map_err(CreateCategoryError::Repository)?
            .iter()
            .any(|category| category.name.is_same_as(name));
        if taken {
            return Err(CreateCategoryError::NameTaken(name.clone()));
        }
        Ok(())
    }
}
