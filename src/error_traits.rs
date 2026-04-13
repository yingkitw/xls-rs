//! Trait-based error handling
//!
//! Provides trait-based error types for better composability and testability.

use std::error::Error as StdError;
use std::fmt;

/// Trait for error types that can provide context
pub trait ErrorContextProvider {
    fn file(&self) -> Option<&str>;
    fn row(&self) -> Option<usize>;
    fn column(&self) -> Option<usize>;
    fn cell_ref(&self) -> Option<&str>;
    fn column_name(&self) -> Option<&str>;
}

/// Trait for error types that can be converted to user-friendly messages
pub trait UserFriendlyError {
    fn user_message(&self) -> String;
    fn suggestion(&self) -> Option<String>;
}

/// Trait for error types that support error recovery
pub trait RecoverableError {
    fn can_recover(&self) -> bool;
    fn recovery_action(&self) -> Option<String>;
}

/// Trait for error types that support error categorization
pub trait ErrorCategory {
    fn category(&self) -> ErrorCategoryType;
    fn severity(&self) -> ErrorSeverity;
}

/// Error category types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategoryType {
    Validation,
    IO,
    Format,
    Type,
    Calculation,
    Configuration,
    Network,
    Other,
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Enhanced error type implementing all error traits
#[derive(Debug)]
pub struct TraitBasedError {
    pub message: String,
    pub category: ErrorCategoryType,
    pub severity: ErrorSeverity,
    pub context: ErrorContext,
    pub suggestion: Option<String>,
    pub recovery: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ErrorContext {
    pub file: Option<String>,
    pub row: Option<usize>,
    pub column: Option<usize>,
    pub cell_ref: Option<String>,
    pub column_name: Option<String>,
}

impl ErrorContextProvider for TraitBasedError {
    fn file(&self) -> Option<&str> {
        self.context.file.as_deref()
    }

    fn row(&self) -> Option<usize> {
        self.context.row
    }

    fn column(&self) -> Option<usize> {
        self.context.column
    }

    fn cell_ref(&self) -> Option<&str> {
        self.context.cell_ref.as_deref()
    }

    fn column_name(&self) -> Option<&str> {
        self.context.column_name.as_deref()
    }
}

impl UserFriendlyError for TraitBasedError {
    fn user_message(&self) -> String {
        let mut msg = self.message.clone();

        if let Some(file) = &self.context.file {
            msg.push_str(&format!(" (file: {})", file));
        }

        if let Some(row) = self.context.row {
            msg.push_str(&format!(" (row: {})", row + 1));
        }

        if let Some(col) = self.context.column {
            msg.push_str(&format!(" (column: {})", col + 1));
        }

        msg
    }

    fn suggestion(&self) -> Option<String> {
        self.suggestion.clone()
    }
}

impl RecoverableError for TraitBasedError {
    fn can_recover(&self) -> bool {
        self.recovery.is_some()
    }

    fn recovery_action(&self) -> Option<String> {
        self.recovery.clone()
    }
}

impl ErrorCategory for TraitBasedError {
    fn category(&self) -> ErrorCategoryType {
        self.category
    }

    fn severity(&self) -> ErrorSeverity {
        self.severity
    }
}

impl fmt::Display for TraitBasedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl StdError for TraitBasedError {}

impl TraitBasedError {
    pub fn new(message: String, category: ErrorCategoryType, severity: ErrorSeverity) -> Self {
        Self {
            message,
            category,
            severity,
            context: ErrorContext::default(),
            suggestion: None,
            recovery: None,
        }
    }

    pub fn with_context(mut self, context: ErrorContext) -> Self {
        self.context = context;
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
    }

    pub fn with_recovery(mut self, recovery: String) -> Self {
        self.recovery = Some(recovery);
        self
    }
}

/// Helper to convert anyhow::Error to TraitBasedError
pub trait ToTraitBasedError {
    fn to_trait_error(
        self,
        category: ErrorCategoryType,
        severity: ErrorSeverity,
    ) -> TraitBasedError;
}

impl ToTraitBasedError for anyhow::Error {
    fn to_trait_error(
        self,
        category: ErrorCategoryType,
        severity: ErrorSeverity,
    ) -> TraitBasedError {
        TraitBasedError::new(self.to_string(), category, severity)
    }
}
