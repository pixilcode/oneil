//! Fuzzy search helpers for picking the best string from a candidate list.
//!
//! [`search`] ranks [`SearchResult`] values using [`Ord`]: an exact match wins, then a candidate
//! that appears as a substring of the query (by Levenshtein distance as a tie-breaker), then a
//! plain fuzzy match (lowest edit distance wins).

use std::cmp;

/// Searches for the best match from a list of candidates. Exact matches are preferred over strings
/// that contain the query, which are in turn preferred over fuzzy matches.
#[must_use]
pub fn search<'input>(query: &str, candidates: &[&'input str]) -> Option<SearchResult<'input>> {
    candidates
        .iter()
        .map(|candidate| {
            if query == *candidate {
                SearchResult::Exact(candidate)
            } else {
                SearchResult::Fuzzy {
                    result: candidate,
                    distance: levenshtein_distance(query, candidate),
                }
            }
        })
        .min()
}

/// Calculates the Levenshtein distance between two strings.
///
/// Based on the algorithm described
/// [on Wikipedia](https://en.wikipedia.org/wiki/Levenshtein_distance#Iterative_with_two_matrix_rows).
fn levenshtein_distance(a: &str, b: &str) -> usize {
    // handle easy cases

    if a == b {
        return 0;
    }

    let a_len = a.len();
    let b_len = b.len();

    if a_len == 0 {
        return b_len;
    }

    if b_len == 0 {
        return a_len;
    }

    // initialize the previous and current rows of distance
    let mut prev_row = vec![0; b_len + 1];
    let mut curr_row = vec![0; b_len + 1];

    for (i, a_char) in a.chars().enumerate() {
        // calculate current row distances from the previous row
        curr_row[0] = i + 1;

        for (j, b_char) in b.chars().enumerate() {
            let deletion_cost = prev_row[j + 1] + 1; // delete the j+1th character of a
            let insertion_cost = curr_row[j] + 1; // insert the i+1th character of b
            let substitution_cost = if a_char == b_char {
                prev_row[j]
            } else {
                prev_row[j] + 1
            }; // substitute the i+1th character of a with the j+1th character of b

            curr_row[j + 1] = deletion_cost.min(insertion_cost).min(substitution_cost);
        }

        // make the current row the previous row
        std::mem::swap(&mut prev_row, &mut curr_row);
    }

    prev_row[b_len]
}

/// Outcome of [`search`]: which candidate won and how it was classified.
///
/// The `distance` field stores the Levenshtein distance between the original `query` and the
/// winning `result` string (see [`search`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResult<'input> {
    /// `query` equals this candidate; no distance field.
    Exact(&'input str),
    /// No exact or substring rule matched; `distance` is [`levenshtein_distance`].
    Fuzzy {
        /// Winning candidate.
        result: &'input str,
        /// Levenshtein distance from `query` to `result`.
        distance: usize,
    },
}

impl<'input> SearchResult<'input> {
    /// Returns `Some(result)` if the result is less than or equal to
    /// a given maximum best match distance.
    #[must_use]
    pub const fn some_if_within_distance(self, max_distance: usize) -> Option<&'input str> {
        match self {
            Self::Exact(result) => Some(result),
            Self::Fuzzy { result, distance } if distance <= max_distance => Some(result),
            Self::Fuzzy { .. } => None,
        }
    }
}

/// [`search`] uses this ordering with [`Iterator::min`]: [`SearchResult::Exact`] is smallest,
/// then [`SearchResult::Contains`] ordered by ascending `distance`, then [`SearchResult::Fuzzy`] by
/// ascending `distance`.
impl PartialOrd for SearchResult<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// See [`PartialOrd`] implementation for [`SearchResult`].
impl Ord for SearchResult<'_> {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        // In this implementation, lower is better
        match (self, other) {
            (Self::Exact(_), Self::Exact(_)) => cmp::Ordering::Equal,
            (Self::Exact(_), _) => cmp::Ordering::Less,
            (_, Self::Exact(_)) => cmp::Ordering::Greater,
            (
                Self::Fuzzy {
                    distance: a,
                    result: _,
                },
                Self::Fuzzy {
                    distance: b,
                    result: _,
                },
            ) => a.cmp(b),
        }
    }
}
