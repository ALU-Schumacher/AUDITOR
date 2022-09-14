// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::env;

use bytes::{Bytes, BytesMut};
use rmp_serde::Serializer;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    JobInfo { job_id: u64 },
    Ok,
    Error { msg: String },
}

impl Message {
    #[tracing::instrument(name = "Obtaining Slurm job info from environment")]
    pub fn jobinfo_from_env() -> Result<Message, anyhow::Error> {
        Ok(Message::JobInfo {
            job_id: env::var("SLURM_JOB_ID")?.parse()?,
        })
    }

    pub fn pack(&self) -> Bytes {
        let mut buf = Vec::new();
        self.serialize(&mut Serializer::new(&mut buf)).unwrap();
        Bytes::from(buf)
    }

    pub fn unpack(buf: &BytesMut) -> Result<Self, anyhow::Error> {
        Ok(rmp_serde::from_slice::<Self>(buf)?)
    }
}
