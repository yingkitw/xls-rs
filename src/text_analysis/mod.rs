//! Text analysis operations
//!
//! Provides text analysis capabilities including sentiment analysis,
//! keyword extraction, text statistics, and language detection.

pub mod analyzer;
pub mod helpers;
pub mod types;

// Re-export main types for convenience
pub use analyzer::TextAnalyzer;
pub use types::{
    Importance, Keyword, KeywordResult, LanguageResult, Sentiment, SentimentResult, SentimentWords,
    TextStats,
};
