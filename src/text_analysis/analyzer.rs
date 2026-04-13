//! Main text analyzer implementation

use crate::common::collection;
use std::collections::HashSet;

use super::types::*;

/// Text analyzer
pub struct TextAnalyzer {
    stop_words: HashSet<String>,
    sentiment_words: SentimentWords,
}

impl TextAnalyzer {
    /// Create a new text analyzer
    pub fn new() -> Self {
        Self {
            stop_words: Self::default_stop_words(),
            sentiment_words: Self::default_sentiment_words(),
        }
    }

    /// Analyze text statistics
    pub fn analyze_stats(&self, text: &str) -> TextStats {
        let words = self.extract_words(text);
        let sentences = self.extract_sentences(text);
        let paragraphs = self.extract_paragraphs(text);

        let word_count = words.len();
        let character_count = text.chars().count();
        let sentence_count = sentences.len();
        let paragraph_count = paragraphs.len();

        let avg_word_length = if word_count > 0 {
            words.iter().map(|w| w.len()).sum::<usize>() as f64 / word_count as f64
        } else {
            0.0
        };

        let avg_sentence_length = if sentence_count > 0 {
            words.len() as f64 / sentence_count as f64
        } else {
            0.0
        };

        let readability_score = self.calculate_readability_score(&words, &sentences);

        let unique_words = collection::unique_preserve_order(&words).len();
        let lexical_diversity = if word_count > 0 {
            unique_words as f64 / word_count as f64
        } else {
            0.0
        };

        TextStats {
            word_count,
            character_count,
            sentence_count,
            paragraph_count,
            avg_word_length,
            avg_sentence_length,
            readability_score,
            unique_words,
            lexical_diversity,
        }
    }

    /// Perform sentiment analysis
    pub fn analyze_sentiment(&self, text: &str) -> SentimentResult {
        let words = self.extract_words(text);

        let mut positive_count = 0;
        let mut negative_count = 0;
        let mut neutral_count = 0;

        for word in &words {
            let lower_word = word.to_lowercase();
            if self.sentiment_words.positive.contains(&lower_word) {
                positive_count += 1;
            } else if self.sentiment_words.negative.contains(&lower_word) {
                negative_count += 1;
            } else {
                neutral_count += 1;
            }
        }

        let total = positive_count + negative_count + neutral_count;
        let (positive_score, negative_score, neutral_score) = if total > 0 {
            (
                positive_count as f64 / total as f64,
                negative_count as f64 / total as f64,
                neutral_count as f64 / total as f64,
            )
        } else {
            (0.0, 0.0, 1.0)
        };

        let (sentiment, confidence) =
            if positive_score > negative_score && positive_score > neutral_score {
                (Sentiment::Positive, positive_score)
            } else if negative_score > positive_score && negative_score > neutral_score {
                (Sentiment::Negative, negative_score)
            } else {
                (Sentiment::Neutral, neutral_score)
            };

        SentimentResult {
            sentiment,
            confidence,
            positive_score,
            negative_score,
            neutral_score,
        }
    }

    /// Extract keywords from text
    pub fn extract_keywords(&self, text: &str, max_keywords: usize) -> KeywordResult {
        let words = self.extract_words(text);

        // Filter out stop words and count frequencies
        let mut word_frequencies: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for word in &words {
            let lower_word = word.to_lowercase();
            if !self.stop_words.contains(&lower_word) && word.len() > 2 {
                *word_frequencies.entry(lower_word.clone()).or_insert(0) += 1;
            }
        }

        // Calculate TF-IDF-like scores
        let total_words = words.len();
        let mut keywords: Vec<Keyword> = word_frequencies
            .into_iter()
            .map(|(word, frequency)| {
                let tf = frequency as f64 / total_words as f64;
                let score = tf * (1.0 + word.len() as f64 * 0.1);

                let importance = if frequency >= total_words / 10 {
                    Importance::High
                } else if frequency >= total_words / 20 {
                    Importance::Medium
                } else {
                    Importance::Low
                };

                Keyword {
                    word,
                    score,
                    frequency,
                    importance,
                }
            })
            .collect();

        // Sort by score and frequency
        keywords.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.frequency.cmp(&a.frequency))
        });

        // Limit to max_keywords
        keywords.truncate(max_keywords);
        let total_keywords = keywords.len();

        KeywordResult {
            keywords,
            total_keywords,
        }
    }

    /// Detect language of text (simplified)
    pub fn detect_language(&self, text: &str) -> LanguageResult {
        // This is a very simplified language detection
        // In a real implementation, you'd use proper language detection libraries

        let supported_languages = vec![
            "English".to_string(),
            "Spanish".to_string(),
            "French".to_string(),
            "German".to_string(),
            "Chinese".to_string(),
            "Japanese".to_string(),
        ];

        // Simple heuristics for language detection
        let lower_text = text.to_lowercase();
        let mut language_scores: std::collections::HashMap<String, f64> =
            std::collections::HashMap::new();

        // English indicators
        if lower_text.contains("the ")
            || lower_text.contains(" and ")
            || lower_text.contains(" is ")
        {
            *language_scores.entry("English".to_string()).or_insert(0.0) += 0.3;
        }

        // Spanish indicators
        if lower_text.contains(" el ") || lower_text.contains(" la ") || lower_text.contains(" de ")
        {
            *language_scores.entry("Spanish".to_string()).or_insert(0.0) += 0.3;
        }

        // French indicators
        if lower_text.contains(" le ") || lower_text.contains(" la ") || lower_text.contains(" et ")
        {
            *language_scores.entry("French".to_string()).or_insert(0.0) += 0.3;
        }

        // Default to English if no indicators found
        let (language, confidence) = language_scores
            .into_iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or_else(|| ("English".to_string(), 0.5));

        LanguageResult {
            language,
            confidence,
            supported_languages,
        }
    }
}

impl Default for TextAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
