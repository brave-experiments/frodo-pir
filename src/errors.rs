use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

// ResultBoxedError returns a result of a given type or a boxed error, in order to encapsulate
// generic error types without requiring an explicit implementation for each error type
pub type ResultBoxedError<T> = Result<T, Box<dyn std::error::Error>>;

// ErrorUnexpectedInputSize is assocuated with unexpected input size on types used for the low
// level cryptographic operations
#[derive(Debug)]
pub struct ErrorUnexpectedInputSize {
  details: String,
}

impl ErrorUnexpectedInputSize {
  pub fn new(details: String) -> Self {
    Self { details }
  }
}

impl Display for ErrorUnexpectedInputSize {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    write!(f, "Unexpected input size error: {}", self.details)
  }
}

impl Error for ErrorUnexpectedInputSize {
  fn description(&self) -> &str {
    &self.details
  }
}

// ErrorQueryParamsReused blocks attempts to reuse query parameters that
// were used already.
#[derive(Debug)]
pub struct ErrorQueryParamsReused;
impl Display for ErrorQueryParamsReused {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    write!(
      f,
      "Attempted to reuse query parameters that were used already"
    )
  }
}
impl Error for ErrorQueryParamsReused {}

// ErrorOverflownAdd blocks attempts to overflown addition.
#[derive(Debug)]
pub struct ErrorOverflownAdd;
impl Display for ErrorOverflownAdd {
  fn fmt(&self, f: &mut Formatter) -> FmtResult {
    write!(f, "Attempted to overflow addition")
  }
}
impl Error for ErrorOverflownAdd {}
