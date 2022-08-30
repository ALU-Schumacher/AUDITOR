// Copyright 2021-2022 AUDITOR developers
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

//! Record related types used for deserializing HTTP requests and serializing HTTP responses.

use super::{Component, ComponentTest, ScoreTest, ValidName};
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
    pub site_id: ValidName,
    pub user_id: ValidName,
    pub group_id: ValidName,
    pub components: Vec<Component>,
    pub start_time: DateTime<Utc>,
    pub stop_time: Option<DateTime<Utc>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordUpdate {
    pub record_id: ValidName,
    pub site_id: ValidName,
    pub user_id: ValidName,
    pub group_id: ValidName,
    pub components: Vec<Component>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Record {
    pub record_id: String,
    pub site_id: Option<String>,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub components: Option<Vec<Component>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
    pub runtime: Option<i64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RecordTest {
    pub record_id: Option<String>,
    pub site_id: Option<String>,
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub components: Option<Vec<ComponentTest>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
}

impl RecordAdd {
    pub fn new<T: AsRef<str>>(
        record_id: T,
        site_id: T,
        user_id: T,
        group_id: T,
        components: Vec<Component>,
        start_time: DateTime<Utc>,
    ) -> Result<Self, Error> {
        Ok(RecordAdd {
            record_id: ValidName::parse(record_id.as_ref().to_string())
                .context("Failed to parse record_id.")?,
            site_id: ValidName::parse(site_id.as_ref().to_string())
                .context("Failed to parse site_id.")?,
            user_id: ValidName::parse(user_id.as_ref().to_string())
                .context("Failed to parse user_id.")?,
            group_id: ValidName::parse(group_id.as_ref().to_string())
                .context("Failed to parse group_id.")?,
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

    pub fn with_site_id<T: AsRef<str>>(mut self, site_id: T) -> Self {
        self.site_id = Some(site_id.as_ref().to_string());
        self
    }

    pub fn with_user_id<T: AsRef<str>>(mut self, user_id: T) -> Self {
        self.user_id = Some(user_id.as_ref().to_string());
        self
    }

    pub fn with_group_id<T: AsRef<str>>(mut self, group_id: T) -> Self {
        self.group_id = Some(group_id.as_ref().to_string());
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

        let mut out = RecordTest::new()
            .with_record_id(fakename())
            .with_site_id(fakename())
            .with_user_id(fakename())
            .with_group_id(fakename())
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
            site_id: s_sid,
            user_id: s_uid,
            group_id: s_gid,
            components: s_comp,
            start_time: s_start,
            stop_time: s_stop,
        } = self;
        let Record {
            record_id: o_rid,
            site_id: o_sid,
            user_id: o_uid,
            group_id: o_gid,
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
            && s_sid == o_sid
            && s_uid == o_uid
            && s_gid == o_gid
            && start_diff < chrono::Duration::milliseconds(1)
            && stop
            && ((s_comp.is_none() && o_comp.is_none())
                || (s_comp.as_ref().unwrap().len() == o_comp.as_ref().unwrap().len()
                    && s_comp
                        .as_ref()
                        .unwrap()
                        .iter()
                        .zip(o_comp.as_ref().unwrap().iter())
                        .fold(true, |acc, (a, b)| acc && (a == b))))
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
            site_id: ValidName::parse(value.site_id.unwrap_or_else(|| "".to_string()))?,
            user_id: ValidName::parse(value.user_id.unwrap_or_else(|| "".to_string()))?,
            group_id: ValidName::parse(value.group_id.unwrap_or_else(|| "".to_string()))?,
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
            site_id: ValidName::parse(value.site_id.unwrap_or_else(|| "".to_string()))
                .context("Failed to parse site_id.")?,
            user_id: ValidName::parse(value.user_id.unwrap_or_else(|| "".to_string()))
                .context("Failed to parse user_id.")?,
            group_id: ValidName::parse(value.group_id.unwrap_or_else(|| "".to_string()))
                .context("Failed to parse group_id.")?,
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
            site_id: ValidName::parse(value.site_id.unwrap_or_else(|| "".to_string()))?,
            user_id: ValidName::parse(value.user_id.unwrap_or_else(|| "".to_string()))?,
            group_id: ValidName::parse(value.group_id.unwrap_or_else(|| "".to_string()))?,
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
            site_id: ValidName::parse(value.site_id.unwrap_or_else(|| "".to_string()))
                .context("Failed to parse site_id.")?,
            user_id: ValidName::parse(value.user_id.unwrap_or_else(|| "".to_string()))
                .context("Failed to parse user_id.")?,
            group_id: ValidName::parse(value.group_id.unwrap_or_else(|| "".to_string()))
                .context("Failed to parse group_id.")?,
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
        Ok(Record {
            record_id: ValidName::parse(
                value
                    .record_id
                    .ok_or_else(|| anyhow::anyhow!("name is None"))?,
            )?
            .as_ref()
            .to_string(),
            site_id: if let Some(site_id) = value.site_id {
                Some(ValidName::parse(site_id)?.as_ref().to_string())
            } else {
                None
            },
            user_id: if let Some(user_id) = value.user_id {
                Some(ValidName::parse(user_id)?.as_ref().to_string())
            } else {
                None
            },
            group_id: if let Some(group_id) = value.group_id {
                Some(ValidName::parse(group_id)?.as_ref().to_string())
            } else {
                None
            },
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
