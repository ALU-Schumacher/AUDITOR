// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

#[macro_export]
macro_rules! debug_for_error {
    ($error_type:ident) => {
        impl std::fmt::Debug for $error_type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                $crate::error::error_chain_fmt(self, f)
            }
        }
    };
}

#[macro_export]
macro_rules! display_for_error {
    ($error_type:ident, $msg:expr) => {
        impl std::fmt::Display for $error_type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, $msg)
            }
        }
    };
}

#[macro_export]
macro_rules! error_for_error {
    ($error_type:ident) => {
        impl std::error::Error for $error_type {
            fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
                Some(&self.0)
            }
        }
    };
}

#[macro_export]
macro_rules! responseerror_for_error {
    ($error_type:ident, $($field:ident => $code:ident;)*) => {
        impl actix_web::ResponseError for $error_type {
            fn status_code(&self) -> actix_web::http::StatusCode {
                match self {
                    $($error_type::$field(_) => actix_web::http::StatusCode::$code),*
                }
            }
        }
    };
}
