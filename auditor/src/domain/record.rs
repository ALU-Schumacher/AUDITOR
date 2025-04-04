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
use rand::Rng;
use serde::{Deserialize, Serialize};

/// `RecordAdd` represents a single accountable unit that is added to Auditor.
///
/// Use the constructor to build a new record. A stop time can be added with the `with_stop_time()`
/// method.
///
/// # Note
/// All strings must not include the characters `/()"<>\{}`.
///
/// When created using the constructor,
/// the record is already valid in terms of all checks that
/// Auditor performs when receiving it.
///
/// # Examples
///
/// Create a record with a valid ID:
///
/// ```
/// # use auditor::domain::{Component, RecordAdd, Score};
/// # use std::collections::HashMap;
/// use chrono::{DateTime, TimeZone, Utc};
///
/// # fn main() -> Result<(), anyhow::Error> {
/// let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
///
/// let component_cpu = Component::new("CPU", 10)?
///     .with_score(Score::new("HEPSPEC06", 9.2)?);
/// let component_mem = Component::new("MEM", 32)?;
/// let components = vec![component_cpu, component_mem];
///
/// let mut meta = HashMap::new();
/// meta.insert("site_id", vec!["site1"]);
/// meta.insert("features", vec!["ssd", "gpu"]);
///
/// let record = RecordAdd::new("123456", meta, components, start_time)?;
/// # Ok(())
/// # }
/// ```
///
/// Create a record with a valid ID and a stop time:
///
/// ```
/// # use auditor::domain::{Component, RecordAdd, Score};
/// # use chrono::{DateTime, TimeZone, Utc};
/// # use std::collections::HashMap;
/// #
/// # fn main() -> Result<(), anyhow::Error> {
/// let start_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();
/// let stop_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
///
/// # let component_cpu = Component::new("CPU", 10)?
/// #     .with_score(Score::new("HEPSPEC06", 9.2)?);
/// # let component_mem = Component::new("MEM", 32)?;
/// # let components = vec![component_cpu, component_mem];
/// #
/// # let mut meta = HashMap::new();
/// # meta.insert("site_id", vec!["site1"]);
/// # meta.insert("features", vec!["ssd", "gpu"]);
/// #
/// let record = RecordAdd::new("123456", meta, components, start_time)?
///     .with_stop_time(stop_time);
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordAdd {
    /// Unique identifier of the record.
    pub record_id: ValidName,
    /// Meta information, a collection of key value pairs in the form of `String` -> `Vec<String>`.
    pub meta: Option<ValidMeta>,
    /// List of components that are accounted for.
    pub components: Vec<Component>,
    /// Start time of the record.
    pub start_time: DateTime<Utc>,
    /// Stop time of the record.
    pub stop_time: Option<DateTime<Utc>>,
}

/// `RecordUpdate` represents a single accountable unit that is used to set the `stop_time` of a
/// [`Record`].
///
/// Initially, records are added to Auditor by pushing a [`RecordAdd`], where the `stop_time` field
/// is optional. To later set the `stop_time` of the record, push a `RecordUpdate` with the same
/// `record_id` to auditor.
///
/// Use the constructor to build a new record.
///
/// # Note
/// All strings must not include the characters `/()"<>\{}`.
///
/// When created using the constructor,
/// the record is already valid in terms of all checks that
/// Auditor performs when receiving it.
///
/// Currently, only the `stop_time` can be updated.
/// Setting other fields such as `meta` or `components` has no effect.
///
/// # Examples
///
/// Create a valid record
///
/// ```
/// # use auditor::domain::{Component, RecordUpdate};
/// # use std::collections::HashMap;
/// use chrono::{DateTime, TimeZone, Utc};
///
/// # fn main() -> Result<(), anyhow::Error> {
/// let stop_time: DateTime<Utc> = Utc.with_ymd_and_hms(2023, 1, 1, 12, 0, 0).unwrap();
/// let record = RecordUpdate::new("123456", HashMap::new(), Vec::new(), stop_time)?;
/// # Ok(())
/// # }
/// ```

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RecordUpdate {
    /// Unique identifier of the record.
    pub record_id: ValidName,
    /// Meta information, a collection of key value pairs in the form of `String` -> `Vec<String>`.
    pub meta: Option<ValidMeta>,
    /// List of components that are accounted for.
    pub components: Vec<Component>,
    /// Start time of the record.
    pub start_time: Option<DateTime<Utc>>,
    /// Stop time of the record.
    pub stop_time: DateTime<Utc>,
}

/// A `Record` represents a single accountable unit.
///
/// Records can be sent to and received from Auditor with the
/// [`AuditorClient`](../../auditor_client/index.html) crate.
/// When initially inserting a record in Auditor, the record is represented as [`RecordAdd`].
/// The `stop_time` can be updated at a later time by pushing a [`RecordUpdate`] to Auditor.
///
/// Records that are retrieved from Auditor are returned as `Record`.
///
/// # Example
///
/// Retrieve all records from Auditor:
///
/// ```ignore
/// # use auditor_client::{AuditorClientBuilder, ClientError};
/// #
/// # fn main() -> Result<(), ClientError> {
/// // Create client by using the builder
/// let client = AuditorClientBuilder::new()
///     .address(&"localhost", 8000)
///     .timeout(20)
///     .build()?;
///
/// // Get all records
/// let records = client.get();
/// # Ok(())
/// # }
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Record {
    /// Unique identifier of the record.
    pub record_id: String,
    /// Meta information, a collection of key value pairs in the form of `String` -> `Vec<String>`.
    pub meta: Option<Meta>,
    /// List of components that are accounted for.
    pub components: Option<Vec<Component>>,
    /// Start time of the record.
    pub start_time: Option<DateTime<Utc>>,
    /// Stop time of the record.
    pub stop_time: Option<DateTime<Utc>>,
    /// Runtime of the record, i.e. the difference between stop and start time.
    pub runtime: Option<i64>,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, sqlx::FromRow)]
pub struct RecordDatabase {
    pub record_id: String,
    pub meta: Option<serde_json::Value>,
    pub components: Option<serde_json::Value>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
    pub runtime: Option<i64>,
}

#[doc(hidden)]
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RecordTest {
    pub record_id: Option<String>,
    pub meta: Option<HashMap<String, Vec<String>>>,
    pub components: Option<Vec<ComponentTest>>,
    pub start_time: Option<DateTime<Utc>>,
    pub stop_time: Option<DateTime<Utc>>,
}

impl RecordAdd {
    /// Constructor.
    ///
    /// # Errors
    ///
    /// * [`anyhow::Error`] - If there was an invalid character (`/()"<>\{}`) in the `record_id` or the
    ///   `meta` information.
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

    /// Set the stop time to the record.
    #[must_use]
    pub fn with_stop_time(mut self, stop_time: DateTime<Utc>) -> Self {
        self.stop_time = Some(stop_time);
        self
    }
}

impl RecordUpdate {
    /// Constructor.
    ///
    /// # Errors
    ///
    /// * [`anyhow::Error`] - If there was an invalid character (`/()"<>\{}`) in the `record_id` or the
    ///   `meta` information.
    pub fn new<T: AsRef<str>>(
        record_id: T,
        meta: HashMap<T, Vec<T>>,
        components: Vec<Component>,
        stop_time: DateTime<Utc>,
    ) -> Result<Self, Error> {
        Ok(RecordUpdate {
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
            start_time: None,
            stop_time,
        })
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
                stop_diff < chrono::Duration::try_milliseconds(1).expect("This should never fail")
            }
            (None, None) => true,
            _ => false,
        };

        s_rid.as_ref().unwrap() == o_rid
            && start_diff < chrono::Duration::try_milliseconds(1).expect("This should never fail")
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

impl From<RecordAdd> for Record {
    fn from(r: RecordAdd) -> Self {
        let runtime = r.stop_time.map(|t| (t - r.start_time).num_seconds());
        Self {
            record_id: r.record_id.to_string(),
            meta: r.meta.map(Into::<Meta>::into),
            components: if r.components.is_empty() {
                None
            } else {
                Some(r.components)
            },
            start_time: Some(r.start_time),
            stop_time: r.stop_time,
            runtime,
        }
    }
}

impl From<RecordUpdate> for Record {
    fn from(r: RecordUpdate) -> Self {
        let runtime = r.start_time.map(|t| (r.stop_time - t).num_seconds());
        Self {
            record_id: r.record_id.to_string(),
            meta: r.meta.map(Into::<Meta>::into),
            components: if r.components.is_empty() {
                None
            } else {
                Some(r.components)
            },
            start_time: r.start_time,
            stop_time: Some(r.stop_time),
            runtime,
        }
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
            serde_json::from_value(meta).ok()
        } else {
            None
        };

        let components = if let Some(components) = components {
            serde_json::from_value(components).ok()
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
