// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

mod component;
mod meta;
mod record;
mod score;
mod validamount;
mod validname;
mod validvalue;

use actix_web::{ResponseError, http::StatusCode};
pub use component::{Component, ComponentTest};
pub use meta::{Meta, ValidMeta};
pub use record::{Record, RecordAdd, RecordDatabase, RecordTest, RecordUpdate};
pub use score::{Score, ScoreTest};
pub use validamount::ValidAmount;
pub use validname::ValidName;
pub use validvalue::ValidValue;

use crate::error::error_chain_fmt;

#[derive(thiserror::Error)]
pub struct ValidationError(String);

impl std::fmt::Debug for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        error_chain_fmt(self, f)
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Validating input failed: {}", self.0)
    }
}

impl ResponseError for ValidationError {
    fn status_code(&self) -> StatusCode {
        StatusCode::BAD_REQUEST
    }
}
