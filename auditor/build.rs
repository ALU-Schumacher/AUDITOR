// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

fn main() {
    // When building in docs.rs, we want to set SQLX_OFFLINE mode to true
    if std::env::var_os("DOCS_RS").is_some() {
        println!("cargo:rustc-env=SQLX_OFFLINE=true");
    }
}
