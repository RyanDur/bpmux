pub mod bayes;
pub mod classifier;
pub mod search;

#[cfg(test)]
mod bayes_tests;
#[cfg(test)]
mod classifier_tests;

pub use classifier::{MessageClass, classify};
pub use search::{SearchFilters, SearchResult, search};
