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
    Integrity {
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

    pub fn integrity(message: impl Into<String>, details: impl Into<String>) -> Self {
        Self::Integrity {
            message: message.into(),
            details: Some(details.into()),
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::Validation { .. } => "VALIDATION",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::Database { .. } => "DB",
            Self::Io { .. } => "IO",
            Self::Integrity { .. } => "INTEGRITY",
        }
    }

    pub fn message(&self) -> &str {
        match self {
            Self::Validation { message }
            | Self::NotFound { message }
            | Self::Conflict { message }
            | Self::Database { message, .. }
            | Self::Io { message, .. }
            | Self::Integrity { message, .. } => message,
        }
    }

    pub fn details(&self) -> Option<&str> {
        match self {
            Self::Validation { .. } | Self::NotFound { .. } | Self::Conflict { .. } => None,
            Self::Database { details, .. }
            | Self::Io { details, .. }
            | Self::Integrity { details, .. } => details.as_deref(),
        }
    }

    /// Texto único para persistência: mensagem e detalhes técnicos, quando houver.
    pub fn message_with_details(&self) -> String {
        match self.details() {
            Some(details) if !details.trim().is_empty() => {
                format!("{} Detalhes: {}", self.message(), details.trim())
            }
            _ => self.message().to_owned(),
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

        assert_eq!(value["code"], "DB");
        assert_eq!(value["message"], "falha no banco");
        assert_eq!(value["details"], "detalhe técnico");
    }

    #[test]
    fn exposes_the_closed_error_codes() {
        assert_eq!(AppError::validation("x").code(), "VALIDATION");
        assert_eq!(AppError::not_found("x").code(), "NOT_FOUND");
        assert_eq!(AppError::conflict("x").code(), "CONFLICT");
        assert_eq!(AppError::database("x", "y").code(), "DB");
        assert_eq!(AppError::io("x", "y").code(), "IO");
        assert_eq!(AppError::integrity("x", "y").code(), "INTEGRITY");
    }

    #[test]
    fn joins_message_and_details_for_persistence() {
        let detailed = AppError::io("o SGS recusou a consulta", "HTTP 406: limite de 10 anos");
        assert_eq!(
            detailed.message_with_details(),
            "o SGS recusou a consulta Detalhes: HTTP 406: limite de 10 anos"
        );

        let plain = AppError::validation("data inválida");
        assert_eq!(plain.message_with_details(), "data inválida");
    }
}
