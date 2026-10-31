use async_trait::async_trait;
use tally_domain::category::CategoryKind;
use uuid::Uuid;

use crate::dto::{CategoryDto, CreateCategoryInput};
use crate::error::AppError;
use crate::validation::normalize_name;

const MAX_NAME_CHARS: usize = 80;

#[derive(Debug, Clone)]
pub struct NewCategory {
    pub id: String,
    pub name: String,
    pub kind: CategoryKind,
}

#[async_trait]
pub trait CategoriesRepository: Send + Sync {
    async fn list(&self, include_archived: bool) -> Result<Vec<CategoryDto>, AppError>;
    async fn create(&self, category: NewCategory) -> Result<CategoryDto, AppError>;
    async fn rename(&self, id: &str, name: &str) -> Result<CategoryDto, AppError>;
    async fn set_archived(&self, id: &str, archived: bool) -> Result<CategoryDto, AppError>;
    async fn delete(&self, id: &str) -> Result<(), AppError>;
}

pub struct CategoriesService<R> {
    repo: R,
}

impl<R: CategoriesRepository> CategoriesService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(&self, include_archived: bool) -> Result<Vec<CategoryDto>, AppError> {
        self.repo.list(include_archived).await
    }

    pub async fn create(&self, input: CreateCategoryInput) -> Result<CategoryDto, AppError> {
        let name = normalize_name(&input.name, "categoria", MAX_NAME_CHARS)?;
        let kind = CategoryKind::parse(&input.kind)
            .ok_or_else(|| AppError::validation("tipo de categoria inválido"))?;

        self.repo
            .create(NewCategory {
                id: Uuid::now_v7().to_string(),
                name,
                kind,
            })
            .await
    }

    pub async fn rename(&self, id: &str, name: &str) -> Result<CategoryDto, AppError> {
        let name = normalize_name(name, "categoria", MAX_NAME_CHARS)?;
        self.repo.rename(id, &name).await
    }

    pub async fn set_archived(&self, id: &str, archived: bool) -> Result<CategoryDto, AppError> {
        self.repo.set_archived(id, archived).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.repo.delete(id).await
    }
}
