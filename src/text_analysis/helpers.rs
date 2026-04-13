//! Helper methods for text analysis

use super::types::SentimentWords;

use std::collections::HashMap;

impl super::analyzer::TextAnalyzer {
    /// Extract words from text
    pub fn extract_words(&self, text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(|c| c.is_alphabetic() || c.is_ascii_digit())
                    .collect::<String>()
            })
            .filter(|word| !word.is_empty())
            .collect()
    }

    /// Extract sentences from text
    pub fn extract_sentences(&self, text: &str) -> Vec<String> {
        text.split(&['.', '!', '?'][..])
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract paragraphs from text
    pub fn extract_paragraphs(&self, text: &str) -> Vec<String> {
        text.split('\n')
            .map(|p| p.trim())
            .filter(|p| !p.is_empty())
            .map(|p| p.to_string())
            .collect()
    }

    /// Calculate readability score (simplified Flesch Reading Ease)
    pub fn calculate_readability_score(&self, words: &[String], sentences: &[String]) -> f64 {
        if sentences.is_empty() || words.is_empty() {
            return 0.0;
        }

        let avg_sentence_length = words.len() as f64 / sentences.len() as f64;
        let avg_syllables = self.estimate_syllables(words) as f64 / words.len() as f64;

        // Simplified Flesch Reading Ease formula
        206.835 - (1.015 * avg_sentence_length) - (84.6 * avg_syllables)
    }

    /// Estimate syllables in words (simplified)
    pub fn estimate_syllables(&self, words: &[String]) -> usize {
        words
            .iter()
            .map(|word| {
                let word_lower = word.to_lowercase();
                let vowel_groups = word_lower
                    .chars()
                    .fold((0, false), |(count, in_vowel_group), c| {
                        let is_vowel = matches!(c, 'a' | 'e' | 'i' | 'o' | 'u' | 'y');
                        if is_vowel && !in_vowel_group {
                            (count + 1, true)
                        } else if !is_vowel {
                            (count, false)
                        } else {
                            (count, true)
                        }
                    })
                    .0;

                // At least one syllable per word
                vowel_groups.max(1)
            })
            .sum()
    }

    /// Calculate word frequencies
    pub fn calculate_word_frequencies(&self, words: &[String]) -> HashMap<String, usize> {
        let mut frequencies = HashMap::new();

        for word in words {
            let lower_word = word.to_lowercase();
            *frequencies.entry(lower_word).or_insert(0) += 1;
        }

        frequencies
    }

    /// Calculate language scores based on word patterns
    pub fn calculate_language_scores(&self, words: &[String]) -> HashMap<String, f64> {
        let mut scores = HashMap::new();

        // This is a very simplified language detection
        // In practice, you'd use n-gram models or statistical methods

        for word in words {
            let lower_word = word.to_lowercase();

            // English indicators
            if lower_word.contains("the") || lower_word.contains("and") || lower_word.contains("is")
            {
                *scores.entry("english".to_string()).or_insert(0.0) += 0.1;
            }

            // Spanish indicators
            if lower_word.contains("el") || lower_word.contains("la") || lower_word.contains("de") {
                *scores.entry("spanish".to_string()).or_insert(0.0) += 0.1;
            }

            // French indicators
            if lower_word.contains("le") || lower_word.contains("la") || lower_word.contains("et") {
                *scores.entry("french".to_string()).or_insert(0.0) += 0.1;
            }

            // German indicators
            if lower_word.contains("der")
                || lower_word.contains("die")
                || lower_word.contains("und")
            {
                *scores.entry("german".to_string()).or_insert(0.0) += 0.1;
            }
        }

        scores
    }

    /// Get default stop words
    pub fn default_stop_words() -> std::collections::HashSet<String> {
        vec![
            "a",
            "an",
            "and",
            "are",
            "as",
            "at",
            "be",
            "but",
            "by",
            "for",
            "if",
            "in",
            "into",
            "is",
            "it",
            "no",
            "not",
            "of",
            "on",
            "or",
            "such",
            "that",
            "the",
            "their",
            "then",
            "there",
            "these",
            "they",
            "this",
            "to",
            "was",
            "will",
            "with",
            "the",
            "is",
            "at",
            "which",
            "on",
            "and",
            "a",
            "an",
            "as",
            "are",
            "was",
            "were",
            "been",
            "be",
            "have",
            "has",
            "had",
            "do",
            "does",
            "did",
            "will",
            "would",
            "should",
            "could",
            "may",
            "might",
            "must",
            "shall",
            "can",
            "cannot",
            "cant",
            "won't",
            "wouldn't",
            "shouldn't",
            "couldn't",
            "mustn't",
            "shan't",
            "mightn't",
            "mustn't",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// Get default sentiment word lists
    pub fn default_sentiment_words() -> SentimentWords {
        let positive: std::collections::HashSet<String> = vec![
            "good",
            "great",
            "excellent",
            "amazing",
            "wonderful",
            "fantastic",
            "awesome",
            "brilliant",
            "outstanding",
            "superb",
            "magnificent",
            "perfect",
            "love",
            "like",
            "enjoy",
            "happy",
            "joy",
            "delight",
            "pleasure",
            "satisfied",
            "pleased",
            "thrilled",
            "excited",
            "enthusiastic",
            "positive",
            "optimistic",
            "hopeful",
            "confident",
            "proud",
            "grateful",
            "thankful",
            "appreciate",
            "beautiful",
            "nice",
            "pretty",
            "handsome",
            "attractive",
            "gorgeous",
            "stunning",
            "elegant",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        let negative: std::collections::HashSet<String> = vec![
            "bad",
            "terrible",
            "awful",
            "horrible",
            "disgusting",
            "disappointing",
            "frustrating",
            "annoying",
            "irritating",
            "angry",
            "mad",
            "furious",
            "enraged",
            "upset",
            "sad",
            "depressed",
            "miserable",
            "unhappy",
            "gloomy",
            "pessimistic",
            "negative",
            "worried",
            "anxious",
            "stressed",
            "overwhelmed",
            "exhausted",
            "tired",
            "bored",
            "uninterested",
            "apathetic",
            "indifferent",
            "ugly",
            "disgusting",
            "repulsive",
            "hideous",
            "grotesque",
            "unpleasant",
            "nasty",
            "vile",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        let neutral: std::collections::HashSet<String> = vec![
            "okay",
            "fine",
            "average",
            "normal",
            "typical",
            "standard",
            "regular",
            "ordinary",
            "common",
            "usual",
            "expected",
            "anticipated",
            "predicted",
            "forecasted",
            "planned",
            "scheduled",
            "arranged",
            "organized",
            "prepared",
            "ready",
            "available",
            "present",
            "existing",
            "current",
            "ongoing",
            "continuing",
            "proceeding",
            "happening",
            "occurring",
            "taking place",
            "underway",
            "in progress",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect();

        SentimentWords {
            positive,
            negative,
            neutral,
        }
    }
}
