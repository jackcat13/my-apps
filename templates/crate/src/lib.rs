//! {{description}}
#![deny(missing_docs)]

use thiserror::Error;

/// Errors returned by this crate.
#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The input contained no words.
    #[error("input is empty")]
    EmptyInput,
}

/// Counts the words in `input`. Placeholder: replace with the crate's real API.
///
/// # Errors
///
/// Returns [`Error::EmptyInput`] when `input` has no words.
///
/// # Examples
///
/// ```
/// assert_eq!({{crate_name}}::word_count("hello world"), Ok(2));
/// ```
pub fn word_count(input: &str) -> Result<usize, Error> {
    match input.split_whitespace().count() {
        0 => Err(Error::EmptyInput),
        n => Ok(n),
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, word_count};

    #[test]
    fn counts_words() {
        assert_eq!(word_count("one two  three"), Ok(3));
    }

    #[test]
    fn rejects_blank_input() {
        assert_eq!(word_count("  \n"), Err(Error::EmptyInput));
    }
}
