use super::category_id::CategoryId;
use super::category_member::CategoryMember;
use super::category_name::CategoryName;
use std::fmt;

#[derive(Debug)]
pub enum CreateCategoryError {
    NameTaken(CategoryName),
    Repository(anyhow::Error),
}

impl fmt::Display for CreateCategoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NameTaken(name) => write!(f, "a category named \"{name}\" already exists"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for CreateCategoryError {}

#[derive(Debug)]
pub enum UpdateCategoryError {
    NotFound(CategoryId),
    NameTaken(CategoryName),
    MemberNotTracked(CategoryMember),
    Repository(anyhow::Error),
}

impl fmt::Display for UpdateCategoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "category {id} not found"),
            Self::NameTaken(name) => write!(f, "a category named \"{name}\" already exists"),
            Self::MemberNotTracked(member) => write!(f, "{member} is not tracked"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for UpdateCategoryError {}

#[derive(Debug)]
pub enum DeleteCategoryError {
    NotFound(CategoryId),
    Repository(anyhow::Error),
}

impl fmt::Display for DeleteCategoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(id) => write!(f, "category {id} not found"),
            Self::Repository(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DeleteCategoryError {}
