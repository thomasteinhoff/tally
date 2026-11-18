use async_trait::async_trait;
use tally_domain::category::CategoryKind;
use uuid::Uuid;

use crate::dto::{CategoryDto, CreateCategoryInput};
use crate::error::AppError;
use crate::validation::normalize_name;

const MAX_NAME_CHARS: usize = 80;
const MAX_ICON_CHARS: usize = 32;

#[derive(Debug, Clone)]
pub struct NewCategory {
    pub id: String,
    pub name: String,
    pub kind: CategoryKind,
    pub icon: String,
}

/// O banco guarda só a chave; o catálogo de paths vive no frontend.
fn normalize_icon(raw: &str) -> Result<String, AppError> {
    let icon = raw.trim().to_lowercase();
    if icon.is_empty() {
        return Ok("generic".to_owned());
    }
    if icon.len() > MAX_ICON_CHARS
        || !icon
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(AppError::validation("ícone de categoria inválido"));
    }
    Ok(icon)
}

#[async_trait]
pub trait CategoriesRepository: Send + Sync {
    async fn list(&self, include_archived: bool) -> Result<Vec<CategoryDto>, AppError>;
    async fn create(&self, category: NewCategory) -> Result<CategoryDto, AppError>;
    async fn update(
        &self,
        id: &str,
        name: &str,
        icon: Option<&str>,
    ) -> Result<CategoryDto, AppError>;
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
        let icon = normalize_icon(&input.icon)?;

        self.repo
            .create(NewCategory {
                id: Uuid::now_v7().to_string(),
                name,
                kind,
                icon,
            })
            .await
    }

    pub async fn update(
        &self,
        id: &str,
        name: &str,
        icon: Option<&str>,
    ) -> Result<CategoryDto, AppError> {
        let name = normalize_name(name, "categoria", MAX_NAME_CHARS)?;
        let icon = icon.map(normalize_icon).transpose()?;
        self.repo.update(id, &name, icon.as_deref()).await
    }

    pub async fn set_archived(&self, id: &str, archived: bool) -> Result<CategoryDto, AppError> {
        self.repo.set_archived(id, archived).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        self.repo.delete(id).await
    }
}
