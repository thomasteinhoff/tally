use serde::{Serialize, Serializer};
use thiserror::Error;
use ts_rs::TS;

/// Forma pública de erro enviada ao frontend via IPC.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../../../src/lib/api/bindings/")]
pub struct AppErrorDto {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Error)]
pub enum AppError {
    #[error("{message}")]
    Validation { message: String },

    #[error("{message}")]
    NotFound { message: String },

    #[error("{message}")]
    Conflict { message: String },

    #[error("{message}")]
    Database {
        message: String,
        details: Option<String>,
    },

    #[error("{message}")]
    Io {
        message: String,
        details: Option<String>,
    },

    #[error("{message}")]
    Bootstrap {
        message: String,
        details: Option<String>,
    },

    #[error("{message}")]
    Internal {
        message: String,
        details: Option<String>,
    },
}

impl AppError {
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
        }
    }

    pub fn database(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Database {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn io(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn bootstrap(message: impl Into<String>) -> Self {
        Self::Bootstrap {
            message: message.into(),
            details: None,
        }
    }

    pub fn bootstrap_with_details(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Bootstrap {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            details: None,
        }
    }

    pub fn internal_with_details(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "VALIDATION",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::Database { .. } => "DATABASE",
            Self::Io { .. } => "IO",
            Self::Bootstrap { .. } => "BOOTSTRAP",
            Self::Internal { .. } => "INTERNAL",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Validation { message }
            | Self::NotFound { message }
            | Self::Conflict { message }
            | Self::Database { message, .. }
            | Self::Io { message, .. }
            | Self::Bootstrap { message, .. }
            | Self::Internal { message, .. } => message,
        }
    }

    pub fn details(&self) -> Option<&str> {
        match self {
            Self::Validation { .. } | Self::NotFound { .. } | Self::Conflict { .. } => None,
            Self::Database { details, .. }
            | Self::Io { details, .. }
            | Self::Bootstrap { details, .. }
            | Self::Internal { details, .. } => details.as_deref(),
        }
    }

    pub fn to_dto(&self) -> AppErrorDto {
        AppErrorDto {
            code: self.code().to_owned(),
            message: self.message().to_owned(),
            details: self.details().map(str::to_owned),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_dto().serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;

    #[test]
    fn serializes_to_stable_error_shape() {
        let error = AppError::database("falha no banco", "detalhe técnico");
        let value = serde_json::to_value(&error).expect("erro deve serializar");

        assert_eq!(value["code"], "DATABASE");
        assert_eq!(value["message"], "falha no banco");
        assert_eq!(value["details"], "detalhe técnico");
    }
}
