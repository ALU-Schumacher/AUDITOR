// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Record related types used for deserializing HTTP requests and serializing HTTP responses.

use std::collections::HashMap;

use super::{Component, ComponentTest, Meta, ScoreTest, ValidMeta, ValidName};
use anyhow::{Context, Error};
use chrono::{DateTime, Utc};
use fake::{Dummy, Fake, Faker, StringFaker};
#[cfg(test)]
use quickcheck;
use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordAdd {
    pub record_id: ValidName,
    pub meta: Option<ValidMeta>,
    pub components: Vec<Component>,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordUpdate {
    pub record_id: ValidName,
    pub meta: Option<ValidMeta>,
    pub components: Vec<Component>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Record {
    pub record_id: String,
    pub meta: Option<Meta>,
    pub components: Option<Vec<Component>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
    pub runtime: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RecordDatabase {
    pub record_id: String,
    pub meta: Option<Vec<(String, Vec<String>)>>,
    pub components: Option<Vec<Component>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
    pub runtime: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RecordTest {
    pub record_id: Option<String>,
    pub meta: Option<HashMap<String, Vec<String>>>,
    pub components: Option<Vec<ComponentTest>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
}

impl RecordAdd {
    pub fn new<T: AsRef<str>>(
        record_id: T,
        meta: HashMap<T, Vec<T>>,
        components: Vec<Component>,
        start_time: DateTime<Utc>,
    ) -> Result<Self, Error> {
        Ok(RecordAdd {
            record_id: ValidName::parse(record_id.as_ref().to_string())
                .context("Failed to parse record_id.")?,
            meta: {
                if meta.is_empty() {
                    None
                } else {
                    Some(meta.try_into()?)
                }
            },
            components,
            start_time,
            stop_time: None,
        })
    }

    #[must_use]
    pub fn with_stop_time(mut self, stop_time: DateTime<Utc>) -> Self {
        self.stop_time = Some(stop_time);
        self
    }
}

impl RecordTest {
    pub fn new() -> Self {
        RecordTest::default()
    }

    pub fn with_record_id<T: AsRef<str>>(mut self, record_id: T) -> Self {
        self.record_id = Some(record_id.as_ref().to_string());
        self
    }

    pub fn with_meta<T: AsRef<str>>(mut self, meta: HashMap<T, Vec<T>>) -> Self {
        self.meta = if meta.is_empty() {
            None
        } else {
            Some(
                meta.into_iter()
                    .map(|(k, v)| {
                        (
                            k.as_ref().to_string(),
                            v.into_iter()
                                .map(|v| v.as_ref().to_string())
                                .collect::<Vec<_>>(),
                        )
                    })
                    .collect(),
            )
        };

        self
    }

    pub fn with_component<T: AsRef<str>>(
        mut self,
        name: T,
        amount: i64,
        scores: Vec<ScoreTest>,
    ) -> Self {
        if self.components.is_none() {
            self.components = Some(vec![])
        }
        self.components.as_mut().unwrap().push(ComponentTest {
            name: Some(name.as_ref().to_string()),
            amount: Some(amount),
            scores,
        });
        self
    }

    pub fn with_start_time<T: AsRef<str>>(mut self, start_time: T) -> Self {
        self.start_time = Some(
            DateTime::parse_from_rfc3339(start_time.as_ref())
                .unwrap()
                .with_timezone(&Utc),
        );
        self
    }

    pub fn with_stop_time<T: AsRef<str>>(mut self, stop_time: T) -> Self {
        self.stop_time = Some(
            DateTime::parse_from_rfc3339(stop_time.as_ref())
                .unwrap()
                .with_timezone(&Utc),
        );
        self
    }
}

impl Dummy<Faker> for RecordTest {
    fn dummy_with_rng<R: Rng + ?Sized>(_: &Faker, rng: &mut R) -> RecordTest {
        let fakename = || -> String {
            StringFaker::with(
                String::from("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789*&^%$#@!~").into_bytes(),
                1..256,
            )
            .fake()
        };
        let fakeamount = || (0..i64::MAX).fake();
        let fakescores = || {
            (0..(0..3u64).fake())
                .map(|_| Faker.fake::<ScoreTest>())
                .collect::<Vec<_>>()
        };
        let fakedate = || -> DateTime<Utc> { Faker.fake() };
        let fakemeta = || -> HashMap<String, Vec<String>> {
            (0..(0..3u64).fake())
                .map(|_| {
                    (
                        fakename(),
                        (0..(1..3u64).fake()).map(|_| fakename()).collect(),
                    )
                })
                .collect()
        };

        let mut out = RecordTest::new()
            .with_record_id(fakename())
            .with_meta(fakemeta())
            .with_start_time(fakedate().to_rfc3339())
            .with_stop_time(fakedate().to_rfc3339());
        for _ in 0..(1..10).fake_with_rng(rng) {
            out = out.with_component(fakename(), fakeamount(), fakescores());
        }
        out
    }
}

impl PartialEq<Record> for RecordTest {
    fn eq(&self, other: &Record) -> bool {
        let RecordTest {
            record_id: s_rid,
            meta: s_meta,
            components: s_comp,
            start_time: s_start,
            stop_time: s_stop,
        } = self;
        let Record {
            record_id: o_rid,
            meta: o_meta,
            components: o_comp,
            start_time: o_start,
            stop_time: o_stop,
            runtime: _,
        } = other;

        // Can't be equal if record ID and start_time are not set in `RecordTest`.
        if s_rid.is_none() || s_start.is_none() {
            return false;
        }

        let s_start = s_start.as_ref().unwrap();
        let o_start = o_start.as_ref().unwrap();

        let start_diff = if s_start > o_start {
            *s_start - *o_start
        } else {
            *o_start - *s_start
        };

        let stop = match (s_stop, o_stop) {
            (Some(s), Some(o)) => {
                let stop_diff = if s > o { *s - *o } else { *o - *s };
                stop_diff < chrono::Duration::milliseconds(1)
            }
            (None, None) => true,
            _ => false,
        };

        s_rid.as_ref().unwrap() == o_rid
            && start_diff < chrono::Duration::milliseconds(1)
            && stop
            && ((s_comp.is_none() && o_comp.is_none())
                || (
                    // s_comp.is_some()
                    //     && o_comp.is_some()
                    //     &&
                    s_comp.as_ref().unwrap().len() == o_comp.as_ref().unwrap().len()
                        && s_comp
                            .as_ref()
                            .unwrap()
                            .iter()
                            .zip(o_comp.as_ref().unwrap().iter())
                            .fold(true, |acc, (a, b)| acc && (a == b))
                ))
            && ((s_meta.is_none() && o_meta.is_none())
                || (s_meta.as_ref().unwrap().len() == o_meta.as_ref().unwrap().len()
                    && s_meta.as_ref().unwrap() == s_meta.as_ref().unwrap()))
    }
}

impl PartialEq<RecordTest> for Record {
    fn eq(&self, other: &RecordTest) -> bool {
        other.eq(self)
    }
}

#[cfg(test)]
impl quickcheck::Arbitrary for RecordTest {
    fn arbitrary(_g: &mut quickcheck::Gen) -> Self {
        Faker.fake()
    }
}

impl TryFrom<RecordTest> for RecordAdd {
    type Error = Error;

    fn try_from(value: RecordTest) -> Result<Self, Self::Error> {
        Ok(RecordAdd {
            record_id: ValidName::parse(
                value
                    .record_id
                    .ok_or_else(|| anyhow::anyhow!("name is None"))?,
            )?,
            meta: value.meta.map(ValidMeta::try_from).transpose()?,
            components: value
                .components
                .unwrap_or_default()
                .into_iter()
                .map(Component::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            start_time: value.start_time.unwrap(),
            stop_time: value.stop_time,
        })
    }
}

impl TryFrom<Record> for RecordAdd {
    type Error = Error;

    fn try_from(value: Record) -> Result<Self, Self::Error> {
        Ok(RecordAdd {
            record_id: ValidName::parse(value.record_id).context("Failed to parse record_id.")?,
            meta: value.meta.map(ValidMeta::try_from).transpose()?,
            components: value
                .components
                .unwrap_or_default()
                .into_iter()
                .map(Component::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            start_time: value
                .start_time
                .ok_or_else(|| anyhow::anyhow!("No start time"))?,
            stop_time: value.stop_time,
        })
    }
}

impl TryFrom<RecordTest> for RecordUpdate {
    type Error = Error;

    fn try_from(value: RecordTest) -> Result<Self, Self::Error> {
        Ok(RecordUpdate {
            record_id: ValidName::parse(
                value
                    .record_id
                    .ok_or_else(|| anyhow::anyhow!("name is None"))?,
            )?,
            meta: value.meta.map(ValidMeta::try_from).transpose()?,
            components: value
                .components
                .unwrap_or_default()
                .into_iter()
                .map(Component::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            start_time: value.start_time,
            stop_time: value.stop_time.unwrap(),
        })
    }
}

impl TryFrom<Record> for RecordUpdate {
    type Error = Error;

    fn try_from(value: Record) -> Result<Self, Self::Error> {
        Ok(RecordUpdate {
            record_id: ValidName::parse(value.record_id).context("Failed to parse record_id.")?,
            meta: value.meta.map(ValidMeta::try_from).transpose()?,
            components: value
                .components
                .unwrap_or_default()
                .into_iter()
                .map(Component::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            start_time: value.start_time,
            stop_time: value.stop_time.unwrap(),
        })
    }
}

impl TryFrom<RecordTest> for Record {
    type Error = Error;

    fn try_from(value: RecordTest) -> Result<Self, Self::Error> {
        let meta: ValidMeta = value.meta.unwrap_or_default().try_into()?;
        Ok(Record {
            record_id: ValidName::parse(
                value
                    .record_id
                    .ok_or_else(|| anyhow::anyhow!("name is None"))?,
            )?
            .as_ref()
            .to_string(),
            meta: Some(meta.into()),
            components: if let Some(components) = value.components {
                Some(
                    components
                        .into_iter()
                        .map(Component::try_from)
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            },
            start_time: value.start_time,
            stop_time: value.stop_time,
            runtime: if let (Some(start), Some(stop)) = (value.start_time, value.stop_time) {
                Some((stop - start).num_seconds())
            } else {
                None
            },
        })
    }
}

impl TryFrom<RecordDatabase> for Record {
    type Error = Error;

    fn try_from(other: RecordDatabase) -> Result<Self, Self::Error> {
        let RecordDatabase {
            record_id,
            meta,
            components,
            start_time,
            stop_time,
            runtime,
        } = other;
        let meta = if let Some(meta) = meta {
            Some(meta.try_into()?)
        } else {
            None
        };
        Ok(Self {
            record_id,
            meta,
            components,
            start_time,
            stop_time,
            runtime,
        })
    }
}
