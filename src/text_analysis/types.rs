//! Types for text analysis operations

use serde::{Deserialize, Serialize};

/// Text analysis statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStats {
    pub word_count: usize,
    pub character_count: usize,
    pub sentence_count: usize,
    pub paragraph_count: usize,
    pub avg_word_length: f64,
    pub avg_sentence_length: f64,
    pub readability_score: f64,
    pub unique_words: usize,
    pub lexical_diversity: f64,
}

/// Sentiment analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentimentResult {
    pub sentiment: Sentiment,
    pub confidence: f64,
    pub positive_score: f64,
    pub negative_score: f64,
    pub neutral_score: f64,
}

/// Sentiment classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Sentiment {
    Positive,
    Negative,
    Neutral,
}

/// Keyword extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordResult {
    pub keywords: Vec<Keyword>,
    pub total_keywords: usize,
}

/// Individual keyword
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keyword {
    pub word: String,
    pub score: f64,
    pub frequency: usize,
    pub importance: Importance,
}

/// Keyword importance level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Importance {
    High,
    Medium,
    Low,
}

/// Language detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageResult {
    pub language: String,
    pub confidence: f64,
    pub supported_languages: Vec<String>,
}

/// Sentiment word lists
#[derive(Debug, Clone)]
pub struct SentimentWords {
    pub positive: std::collections::HashSet<String>,
    pub negative: std::collections::HashSet<String>,
    pub neutral: std::collections::HashSet<String>,
}
