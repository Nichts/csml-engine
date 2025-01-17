use crate::data::ast::Interval;

use serde::{Deserialize, Serialize};
use std::hash::Hash;

////////////////////////////////////////////////////////////////////////////////
// STRUCTURE
////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Default)]
pub struct Position {
    pub flow: String,
    pub interval: Interval,
}

////////////////////////////////////////////////////////////////////////////////
// TRAIT FUNCTION
////////////////////////////////////////////////////////////////////////////////

////////////////////////////////////////////////////////////////////////////////
// PUBLIC FUNCTION
////////////////////////////////////////////////////////////////////////////////

impl Position {
    pub fn new(interval: Interval, flow: &str) -> Self {
        Self {
            flow: flow.to_owned(),
            interval,
        }
    }
}
