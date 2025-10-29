use std::fmt::{self, Debug, Display};

use serde::Deserialize;

pub type JitoResult<T, E> = Result<T, JitoError<E>>;

#[derive(Debug, Deserialize)]
pub enum JitoError<T> {
    BundleError(T),
    TipError(T),
    BlockEngineError(T),
    ValidatorsError(T),
    TransactionsPoolError(T),
    HealthError(T),
    StatisticsError(T),
    SerializationError(T),
    Error(T),
    InsufficientBalance,
    NoArbitrageOpportunity,
}

impl<T> fmt::Display for JitoError<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JitoError::BundleError(msg) => write!(f, "Bundle error: {}", msg),
            JitoError::TipError(msg) => write!(f, "Tip error: {}", msg),
            JitoError::BlockEngineError(msg) => write!(f, "Block engine error: {}", msg),
            JitoError::ValidatorsError(msg) => write!(f, "Validators error: {}", msg),
            JitoError::TransactionsPoolError(msg) => write!(f, "Transactions pool error: {}", msg),
            JitoError::HealthError(msg) => write!(f, "Health error: {}", msg),
            JitoError::StatisticsError(msg) => write!(f, "Statistics error: {}", msg),
            JitoError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            JitoError::InsufficientBalance => write!(f, "Insufficient balance"),
            JitoError::NoArbitrageOpportunity => write!(f, "No arbitrage opportunity found"),
            JitoError::Error(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl<T> std::error::Error for JitoError<T> where T: Display + Debug {}

impl<T> From<T> for JitoError<T> {
    fn from(e: T) -> Self {
        JitoError::SerializationError(e)
    }
}
