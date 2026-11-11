use crate::dto::{BootstrapDto, IntegrityReportDto};
use crate::error::AppError;
use crate::ports::BootstrapRepository;

pub struct BootstrapService<R> {
    repo: R,
    app_version: String,
    db_path: String,
    data_dir: String,
}

impl<R: BootstrapRepository> BootstrapService<R> {
    pub fn new(
        repo: R,
        app_version: impl Into<String>,
        db_path: impl Into<String>,
        data_dir: impl Into<String>,
    ) -> Self {
        Self {
            repo,
            app_version: app_version.into(),
            db_path: db_path.into(),
            data_dir: data_dir.into(),
        }
    }

    pub async fn bootstrap(&self) -> Result<BootstrapDto, AppError> {
        let schema_version = self.repo.schema_version().await?;
        let settings = self.repo.settings().await?;
        let system_accounts = self.repo.system_account_count().await?;
        let integrity = self.repo.integrity_check().await?;

        Ok(BootstrapDto {
            app_version: self.app_version.clone(),
            db_path: self.db_path.clone(),
            data_dir: self.data_dir.clone(),
            schema_version,
            settings,
            system_accounts,
            integrity,
        })
    }

    pub async fn integrity_check(&self) -> Result<IntegrityReportDto, AppError> {
        self.repo.integrity_check().await
    }

    pub async fn settings(&self) -> Result<crate::dto::SettingsDto, AppError> {
        self.repo.settings().await
    }

    pub async fn update_settings(
        &self,
        input: crate::dto::UpdateSettingsInput,
    ) -> Result<crate::dto::SettingsDto, AppError> {
        if input.backup_retention == 0 {
            return Err(AppError::validation(
                "mantenha pelo menos um backup na retenção",
            ));
        }

        self.repo.update_settings(input.backup_retention).await
    }
}

#[cfg(test)]
mod tests {
    use super::BootstrapService;
    use crate::dto::{IntegrityReportDto, SettingsDto};
    use crate::error::AppError;
    use crate::ports::BootstrapRepository;
    use async_trait::async_trait;

    struct FakeRepository;

    #[async_trait]
    impl BootstrapRepository for FakeRepository {
        async fn schema_version(&self) -> Result<i64, AppError> {
            Ok(1)
        }

        async fn settings(&self) -> Result<SettingsDto, AppError> {
            Ok(SettingsDto {
                backup_retention: 10,
                first_run_completed: false,
                last_backup_at: None,
            })
        }

        async fn update_settings(&self, backup_retention: u32) -> Result<SettingsDto, AppError> {
            Ok(SettingsDto {
                backup_retention,
                first_run_completed: false,
                last_backup_at: None,
            })
        }

        async fn system_account_count(&self) -> Result<i64, AppError> {
            Ok(4)
        }

        async fn integrity_check(&self) -> Result<IntegrityReportDto, AppError> {
            Ok(IntegrityReportDto {
                ok: true,
                issues: Vec::new(),
            })
        }
    }

    #[tokio::test]
    async fn composes_bootstrap_from_repository() {
        let service = BootstrapService::new(FakeRepository, "0.1.0", "tally.db", "Tally");
        let dto = service.bootstrap().await.expect("bootstrap deve funcionar");

        assert_eq!(dto.app_version, "0.1.0");
        assert_eq!(dto.schema_version, 1);
        assert_eq!(dto.system_accounts, 4);
        assert!(dto.integrity.ok);
    }
}
