//! String module for the Flux standard library
//! 
//! This module provides string manipulation functions, formatting, interpolation,
//! regular expression support, and Unicode-aware string operations.

use std::fmt;
use regex::Regex;
use crate::runtime::result::{FluxResult, FluxError};

/// Represents different types of string errors that can occur
#[derive(Debug, Clone)]
pub enum StringError {
    InvalidIndex { index: usize, length: usize },
    InvalidRange { start: usize, end: usize, length: usize },
    InvalidPattern(String),
    EncodingError(String),
    FormatError(String),
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StringError::InvalidIndex { index, length } => {
                write!(f, "Index {} out of bounds for string length {}", index, length)
            },
            StringError::InvalidRange { start, end, length } => {
                write!(f, "Range {}..{} out of bounds for string length {}", start, end, length)
            },
            StringError::InvalidPattern(pattern) => write!(f, "Invalid regex pattern: {}", pattern),
            StringError::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            StringError::FormatError(msg) => write!(f, "Format error: {}", msg),
        }
    }
}

impl From<StringError> for FluxError {
    fn from(error: StringError) -> Self {
        FluxError::Custom(error.to_string())
    }
}

impl From<regex::Error> for StringError {
    fn from(error: regex::Error) -> Self {
        StringError::InvalidPattern(error.to_string())
    }
}

/// Enhanced string type with additional functionality
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FluxString {
    inner: String,
}

impl FluxString {
    /// Creates a new empty FluxString
    pub fn new() -> Self {
        FluxString {
            inner: String::new(),
        }
    }
    
    /// Creates a FluxString from a standard string
    pub fn from_string(s: String) -> Self {
        FluxString { inner: s }
    }
    
    /// Creates a FluxString from a string slice
    pub fn from_str(s: &str) -> Self {
        FluxString {
            inner: s.to_string(),
        }
    }
    
    /// Returns the length of the string in bytes
    pub fn len(&self) -> usize {
        self.inner.len()
    }
    
    /// Returns the length of the string in Unicode characters
    pub fn char_len(&self) -> usize {
        self.inner.chars().count()
    }
    
    /// Returns true if the string is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
    
    /// Returns the underlying string
    pub fn as_str(&self) -> &str {
        &self.inner
    }
    
    /// Converts to a standard String
    pub fn to_string(&self) -> String {
        self.inner.clone()
    }
    
    /// Gets a character at the specified index (Unicode-aware)
    pub fn char_at(&self, index: usize) -> FluxResult<char, StringError> {
        match self.inner.chars().nth(index) {
            Some(ch) => FluxResult::Ok(ch),
            None => FluxResult::Err(StringError::InvalidIndex {
                index,
                length: self.char_len(),
            }),
        }
    }
    
    /// Gets a substring by character indices (Unicode-aware)
    pub fn substring(&self, start: usize, end: usize) -> FluxResult<FluxString, StringError> {
        let char_len = self.char_len();
        
        if start > char_len || end > char_len || start > end {
            return FluxResult::Err(StringError::InvalidRange {
                start,
                end,
                length: char_len,
            });
        }
        
        let substring: String = self.inner
            .chars()
            .skip(start)
            .take(end - start)
            .collect();
            
        FluxResult::Ok(FluxString::from_string(substring))
    }
    
    /// Splits the string by a delimiter
    pub fn split(&self, delimiter: &str) -> Vec<FluxString> {
        self.inner
            .split(delimiter)
            .map(|s| FluxString::from_str(s))
            .collect()
    }
    
    /// Splits the string by whitespace
    pub fn split_whitespace(&self) -> Vec<FluxString> {
        self.inner
            .split_whitespace()
            .map(|s| FluxString::from_str(s))
            .collect()
    }
    
    /// Splits the string into lines
    pub fn lines(&self) -> Vec<FluxString> {
        self.inner
            .lines()
            .map(|s| FluxString::from_str(s))
            .collect()
    }
    
    /// Joins a collection of strings with this string as delimiter
    pub fn join(&self, strings: &[FluxString]) -> FluxString {
        let str_refs: Vec<&str> = strings.iter().map(|s| s.as_str()).collect();
        FluxString::from_string(str_refs.join(&self.inner))
    }
    
    /// Trims whitespace from both ends
    pub fn trim(&self) -> FluxString {
        FluxString::from_str(self.inner.trim())
    }
    
    /// Trims whitespace from the start
    pub fn trim_start(&self) -> FluxString {
        FluxString::from_str(self.inner.trim_start())
    }
    
    /// Trims whitespace from the end
    pub fn trim_end(&self) -> FluxString {
        FluxString::from_str(self.inner.trim_end())
    }
    
    /// Trims specific characters from both ends
    pub fn trim_matches(&self, pattern: char) -> FluxString {
        FluxString::from_str(self.inner.trim_matches(pattern))
    }
    
    /// Converts to lowercase
    pub fn to_lowercase(&self) -> FluxString {
        FluxString::from_string(self.inner.to_lowercase())
    }
    
    /// Converts to uppercase
    pub fn to_uppercase(&self) -> FluxString {
        FluxString::from_string(self.inner.to_uppercase())
    }
    
    /// Capitalizes the first character
    pub fn capitalize(&self) -> FluxString {
        let mut chars: Vec<char> = self.inner.chars().collect();
        if let Some(first) = chars.first_mut() {
            *first = first.to_uppercase().next().unwrap_or(*first);
        }
        FluxString::from_string(chars.into_iter().collect())
    }
    
    /// Converts to title case (capitalizes each word)
    pub fn to_title_case(&self) -> FluxString {
        let result = self.inner
            .split_whitespace()
            .map(|word| {
                let mut chars: Vec<char> = word.chars().collect();
                if let Some(first) = chars.first_mut() {
                    *first = first.to_uppercase().next().unwrap_or(*first);
                }
                chars.into_iter().collect::<String>()
            })
            .collect::<Vec<String>>()
            .join(" ");
        FluxString::from_string(result)
    }
    
    /// Checks if the string starts with a prefix
    pub fn starts_with(&self, prefix: &str) -> bool {
        self.inner.starts_with(prefix)
    }
    
    /// Checks if the string ends with a suffix
    pub fn ends_with(&self, suffix: &str) -> bool {
        self.inner.ends_with(suffix)
    }
    
    /// Checks if the string contains a substring
    pub fn contains(&self, substring: &str) -> bool {
        self.inner.contains(substring)
    }
    
    /// Finds the first occurrence of a substring
    pub fn find(&self, substring: &str) -> Option<usize> {
        self.inner.find(substring)
    }
    
    /// Finds the last occurrence of a substring
    pub fn rfind(&self, substring: &str) -> Option<usize> {
        self.inner.rfind(substring)
    }
    
    /// Replaces all occurrences of a pattern with a replacement
    pub fn replace(&self, pattern: &str, replacement: &str) -> FluxString {
        FluxString::from_string(self.inner.replace(pattern, replacement))
    }
    
    /// Replaces the first n occurrences of a pattern
    pub fn replacen(&self, pattern: &str, replacement: &str, count: usize) -> FluxString {
        FluxString::from_string(self.inner.replacen(pattern, replacement, count))
    }
    
    /// Repeats the string n times
    pub fn repeat(&self, count: usize) -> FluxString {
        FluxString::from_string(self.inner.repeat(count))
    }
    
    /// Pads the string to a minimum width with spaces
    pub fn pad_left(&self, width: usize) -> FluxString {
        if self.char_len() >= width {
            self.clone()
        } else {
            let padding = " ".repeat(width - self.char_len());
            FluxString::from_string(format!("{}{}", padding, self.inner))
        }
    }
    
    /// Pads the string to a minimum width with spaces on the right
    pub fn pad_right(&self, width: usize) -> FluxString {
        if self.char_len() >= width {
            self.clone()
        } else {
            let padding = " ".repeat(width - self.char_len());
            FluxString::from_string(format!("{}{}", self.inner, padding))
        }
    }
    
    /// Centers the string in a field of given width
    pub fn center(&self, width: usize) -> FluxString {
        let len = self.char_len();
        if len >= width {
            self.clone()
        } else {
            let total_padding = width - len;
            let left_padding = total_padding / 2;
            let right_padding = total_padding - left_padding;
            FluxString::from_string(format!(
                "{}{}{}",
                " ".repeat(left_padding),
                self.inner,
                " ".repeat(right_padding)
            ))
        }
    }
    
    /// Reverses the string (Unicode-aware)
    pub fn reverse(&self) -> FluxString {
        let reversed: String = self.inner.chars().rev().collect();
        FluxString::from_string(reversed)
    }
    
    /// Checks if the string is numeric
    pub fn is_numeric(&self) -> bool {
        !self.inner.is_empty() && self.inner.chars().all(|c| c.is_numeric())
    }
    
    /// Checks if the string is alphabetic
    pub fn is_alphabetic(&self) -> bool {
        !self.inner.is_empty() && self.inner.chars().all(|c| c.is_alphabetic())
    }
    
    /// Checks if the string is alphanumeric
    pub fn is_alphanumeric(&self) -> bool {
        !self.inner.is_empty() && self.inner.chars().all(|c| c.is_alphanumeric())
    }
    
    /// Checks if the string is ASCII
    pub fn is_ascii(&self) -> bool {
        self.inner.is_ascii()
    }
    
    /// Appends another string
    pub fn append(&mut self, other: &FluxString) {
        self.inner.push_str(&other.inner);
    }
    
    /// Prepends another string
    pub fn prepend(&mut self, other: &FluxString) {
        self.inner = format!("{}{}", other.inner, self.inner);
    }
    
    /// Inserts a string at the specified position
    pub fn insert_str(&mut self, index: usize, s: &str) -> FluxResult<(), StringError> {
        if index > self.len() {
            FluxResult::Err(StringError::InvalidIndex {
                index,
                length: self.len(),
            })
        } else {
            self.inner.insert_str(index, s);
            FluxResult::Ok(())
        }
    }
}

impl Default for FluxString {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FluxString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl From<String> for FluxString {
    fn from(s: String) -> Self {
        FluxString::from_string(s)
    }
}

impl From<&str> for FluxString {
    fn from(s: &str) -> Self {
        FluxString::from_str(s)
    }
}

/// Regular expression support
pub struct FluxRegex {
    regex: Regex,
}

impl FluxRegex {
    /// Creates a new regex from a pattern
    pub fn new(pattern: &str) -> FluxResult<Self, StringError> {
        match Regex::new(pattern) {
            Ok(regex) => FluxResult::Ok(FluxRegex { regex }),
            Err(e) => FluxResult::Err(StringError::from(e)),
        }
    }
    
    /// Tests if the regex matches the string
    pub fn is_match(&self, text: &str) -> bool {
        self.regex.is_match(text)
    }
    
    /// Finds the first match in the string
    pub fn find(&self, text: &str) -> Option<(usize, usize)> {
        self.regex.find(text).map(|m| (m.start(), m.end()))
    }
    
    /// Finds all matches in the string
    pub fn find_all(&self, text: &str) -> Vec<(usize, usize)> {
        self.regex
            .find_iter(text)
            .map(|m| (m.start(), m.end()))
            .collect()
    }
    
    /// Captures groups from the first match
    pub fn captures(&self, text: &str) -> Option<Vec<String>> {
        self.regex.captures(text).map(|caps| {
            caps.iter()
                .map(|m| m.map_or(String::new(), |m| m.as_str().to_string()))
                .collect()
        })
    }
    
    /// Replaces the first match with a replacement string
    pub fn replace(&self, text: &str, replacement: &str) -> String {
        self.regex.replace(text, replacement).to_string()
    }
    
    /// Replaces all matches with a replacement string
    pub fn replace_all(&self, text: &str, replacement: &str) -> String {
        self.regex.replace_all(text, replacement).to_string()
    }
    
    /// Splits the string by the regex pattern
    pub fn split(&self, text: &str) -> Vec<String> {
        self.regex
            .split(text)
            .map(|s| s.to_string())
            .collect()
    }
}

/// String formatting utilities

/// Formats a string with positional arguments
pub fn format_string(template: &str, args: &[&dyn fmt::Display]) -> FluxResult<String, StringError> {
    // Simple implementation - in a real system this would be more sophisticated
    let mut result = template.to_string();
    
    for (i, arg) in args.iter().enumerate() {
        let placeholder = format!("{{{}}}", i);
        let replacement = format!("{}", arg);
        result = result.replace(&placeholder, &replacement);
    }
    
    FluxResult::Ok(result)
}

/// String interpolation (simplified version)
pub fn interpolate(template: &str, values: &std::collections::HashMap<String, String>) -> String {
    let mut result = template.to_string();
    
    for (key, value) in values {
        let placeholder = format!("{{{}}}", key);
        result = result.replace(&placeholder, value);
    }
    
    result
}

/// Utility functions for common string operations

/// Joins strings with a delimiter
pub fn join_strings(strings: &[&str], delimiter: &str) -> String {
    strings.join(delimiter)
}

/// Splits a string and returns FluxString vector
pub fn split_string(s: &str, delimiter: &str) -> Vec<FluxString> {
    s.split(delimiter)
        .map(FluxString::from_str)
        .collect()
}

/// Checks if a string is a valid email address (simple check)
pub fn is_valid_email(email: &str) -> bool {
    let email_regex = r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$";
    match Regex::new(email_regex) {
        Ok(regex) => regex.is_match(email),
        Err(_) => false,
    }
}

/// Checks if a string is a valid URL (simple check)
pub fn is_valid_url(url: &str) -> bool {
    let url_regex = r"^https?://[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}(/.*)?$";
    match Regex::new(url_regex) {
        Ok(regex) => regex.is_match(url),
        Err(_) => false,
    }
}

/// Escapes HTML characters in a string
pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// Unescapes HTML characters in a string
pub fn unescape_html(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#x27;", "'")
}

/// Converts a string to kebab-case
pub fn to_kebab_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_upper = false;
    
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_was_upper {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_was_upper = true;
        } else if ch.is_whitespace() || ch == '_' {
            if !result.is_empty() && !result.ends_with('-') {
                result.push('-');
            }
            prev_was_upper = false;
        } else {
            result.push(ch);
            prev_was_upper = false;
        }
    }
    
    result
}

/// Converts a string to snake_case
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    let mut prev_was_upper = false;
    
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_was_upper {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
            prev_was_upper = true;
        } else if ch.is_whitespace() || ch == '-' {
            if !result.is_empty() && !result.ends_with('_') {
                result.push('_');
            }
            prev_was_upper = false;
        } else {
            result.push(ch);
            prev_was_upper = false;
        }
    }
    
    result
}

/// Converts a string to camelCase
pub fn to_camel_case(s: &str) -> String {
    let words: Vec<&str> = s.split(|c: char| c.is_whitespace() || c == '-' || c == '_').collect();
    let mut result = String::new();
    
    for (i, word) in words.iter().enumerate() {
        if word.is_empty() {
            continue;
        }
        
        if i == 0 {
            result.push_str(&word.to_lowercase());
        } else {
            let mut chars: Vec<char> = word.to_lowercase().chars().collect();
            if let Some(first) = chars.first_mut() {
                *first = first.to_uppercase().next().unwrap_or(*first);
            }
            result.push_str(&chars.into_iter().collect::<String>());
        }
    }
    
    result
}

/// Converts a string to PascalCase
pub fn to_pascal_case(s: &str) -> String {
    let words: Vec<&str> = s.split(|c: char| c.is_whitespace() || c == '-' || c == '_').collect();
    let mut result = String::new();
    
    for word in words {
        if word.is_empty() {
            continue;
        }
        
        let mut chars: Vec<char> = word.to_lowercase().chars().collect();
        if let Some(first) = chars.first_mut() {
            *first = first.to_uppercase().next().unwrap_or(*first);
        }
        result.push_str(&chars.into_iter().collect::<String>());
    }
    
    result
}

/// Calculates the Levenshtein distance between two strings
pub fn levenshtein_distance(s1: &str, s2: &str) -> usize {
    let chars1: Vec<char> = s1.chars().collect();
    let chars2: Vec<char> = s2.chars().collect();
    let len1 = chars1.len();
    let len2 = chars2.len();
    
    if len1 == 0 {
        return len2;
    }
    if len2 == 0 {
        return len1;
    }
    
    let mut matrix = vec![vec![0; len2 + 1]; len1 + 1];
    
    for i in 0..=len1 {
        matrix[i][0] = i;
    }
    for j in 0..=len2 {
        matrix[0][j] = j;
    }
    
    for i in 1..=len1 {
        for j in 1..=len2 {
            let cost = if chars1[i - 1] == chars2[j - 1] { 0 } else { 1 };
            matrix[i][j] = std::cmp::min(
                std::cmp::min(matrix[i - 1][j] + 1, matrix[i][j - 1] + 1),
                matrix[i - 1][j - 1] + cost,
            );
        }
    }
    
    matrix[len1][len2]
}