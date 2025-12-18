use std::collections::HashMap;

use chrono::{DateTime, Utc};
use jiff::Timestamp;
use k8s_openapi::api::core::v1::Pod;
use kube::api::ListParams;

use super::RecordCollector;
use crate::{
    CONFIG,
    constants::{KEY_NAMESPACE, KEY_PODNAME, KEY_STATUS},
};
use kubernetes::KubeApi;

use auditor::domain::{Component, RecordAdd};

pub struct KapiCollector {
    api: KubeApi<Pod>,
}

impl KapiCollector {
    #[tracing::instrument(name = "Create KAPI Collector", level = "debug")]
    pub async fn new() -> Self {
        let api = KubeApi::new(
            &CONFIG
                .get()
                .unwrap()
                .job_filter
                .namespace
                .iter()
                .map(<String as AsRef<str>>::as_ref)
                .collect::<Vec<_>>(),
        )
        .await;
        Self { api }
    }
}

impl RecordCollector for KapiCollector {
    #[tracing::instrument(name = "Retrieve records from Kubernetes", skip(self))]
    async fn list_records(
        &self,
        lastcheck: &Option<DateTime<Utc>>,
    ) -> anyhow::Result<Vec<RecordAdd>> {
        let labelfilter = CONFIG.get().unwrap().job_filter.labels.join(",");
        let lp = ListParams::default().labels(&labelfilter);
        let pods = self.api.list(&lp).await?;
        let mut records = Vec::with_capacity(pods.len());
        for p in pods {
            let r = match pod_to_record(p) {
                Ok(Some(r)) => r,
                Ok(None) => continue,
                Err(e) => {
                    tracing::error!("Cannot parse pod: {}", e);
                    continue;
                }
            };
            // Works since None < Some(_)
            if &r.stop_time > lastcheck {
                records.push(r);
            }
        }
        Ok(records)
    }
}

#[tracing::instrument(
    name = "Converting Pod to Record",
    level = "debug",
    skip(pod),
    fields(podname = pod.metadata.name)
)]
pub(crate) fn pod_to_record(pod: Pod) -> anyhow::Result<Option<RecordAdd>> {
    let config = CONFIG.get().unwrap();

    // Get basic info about pod
    let Pod {
        metadata,
        spec: _,
        status,
    } = &pod;
    let name = metadata
        .name
        .clone()
        .ok_or(anyhow::anyhow!("Pod has no name: {:?}", &pod))?;
    let namespace = metadata
        .namespace
        .clone()
        .ok_or(anyhow::anyhow!("Pod {} has no namespace", name))?;
    let uid = metadata
        .uid
        .clone()
        .ok_or(anyhow::anyhow!("Pod {} has no uid", name))?;
    //let spec = spec.ok_or(
    //    anyhow::Error::msg(format!("Pod {} has no spec field", name))
    //)?;
    let status = status
        .as_ref()
        .ok_or(anyhow::anyhow!("Pod {} has no status field", name))?;

    // Check if pod is finished
    let phase = status
        .phase
        .clone()
        .ok_or(anyhow::anyhow!("Pod {} has no phase field", name))?;
    // For phases see https://kubernetes.io/docs/concepts/workloads/pods/pod-lifecycle/
    if phase.eq_ignore_ascii_case("Pending") || phase.eq_ignore_ascii_case("Running") {
        tracing::debug!("Pod {} not finished. Skipping.", name);
        return Ok(None);
    };
    if phase.eq_ignore_ascii_case("Unknown") {
        anyhow::bail!("Phase of pod {} unknown.", name);
    };

    // NOTE: Using the pod time, we will inclunde container creation
    let start_time = status
        .start_time
        .clone()
        .ok_or(anyhow::anyhow!("Pod {} has no start_time", name))?
        .0;
    let stop_time = get_stop_time(&pod)
        .map_err(|e| {
            tracing::warn!("Pod {} has no stop time: {}", name, e);
            e
        })
        .ok();

    // Fill Meta
    let mut meta = HashMap::new();
    meta.insert(KEY_PODNAME.to_string(), vec![name.clone()]);
    meta.insert(KEY_NAMESPACE.to_string(), vec![namespace.clone()]);
    meta.insert(KEY_STATUS.to_string(), vec![phase]);

    let components = get_components(&pod);
    if let Err(ref e) = components {
        tracing::warn!("Cannot retrieve components of {}: {}", name, e);
    }

    let record = RecordAdd::new(
        format!("{}-{}-{}-{}", config.record_prefix, namespace, name, uid),
        meta,
        components.unwrap_or_default(),
        DateTime::from_timestamp_millis(start_time.as_millisecond()).expect(""),
    )?;
    Ok(Some(if let Some(t) = stop_time {
        record.with_stop_time(DateTime::from_timestamp_millis(t.as_millisecond()).expect(""))
    } else {
        record
    }))
}

/// Return the stoptime of a pod. Since Pod objects don't have a stoptime
/// we need to go through the corresponding containers.
#[tracing::instrument(
    name = "Read Pod stop time",
    level = "trace",
    skip(pod),
    fields(podname = pod.metadata.name)
)]
fn get_stop_time(pod: &Pod) -> anyhow::Result<Timestamp> {
    let container_statuses = pod
        .status
        .as_ref()
        .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?
        .container_statuses
        .as_ref()
        .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?;
    let num = container_statuses.len();
    let stop_times: Vec<_> = container_statuses
        .iter()
        .filter_map(|cstatus| {
            cstatus
                .state
                .as_ref()
                .and_then(|x| x.terminated.as_ref())
                .and_then(|x| x.finished_at.as_ref())
        })
        .collect();
    if stop_times.len() != num {
        Err(anyhow::anyhow!("Container status incomplete {}", line!()))
    } else {
        Ok(stop_times
            .iter()
            .max()
            .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?
            .0)
    }
}

// Kubernetes uses a granularity of "millicpus", so we return "millis"
#[tracing::instrument(name = "Parsing quantity", level = "trace")]
fn parse_si(s: &str) -> anyhow::Result<i64> {
    let err = || anyhow::anyhow!(format!("Cannot parse {}", s));
    if !s.is_ascii() {
        return Err(err());
    };
    let idx = s
        .chars()
        .position(|c| !"0123456789".contains(c))
        .unwrap_or(s.len());
    let (num, fix) = s.split_at(idx);
    let factor = match fix {
        "" => 1000,
        "m" => 1,
        "k" => 1_000_000,
        "Ki" => 1_024_000,
        "M" => 1000_i64.pow(3),
        "Mi" => 1024_i64.pow(2) * 1000,
        "G" => 1000_i64.pow(4),
        "Gi" => 1024_i64.pow(3) * 1000,
        "T" => 1000_i64.pow(5),
        "Ti" => 1024_i64.pow(4) * 1000,
        _ => return Err(err()),
    };
    Ok(num.parse::<i64>()? * factor)
}

#[tracing::instrument(
    name = "Read Pod components",
    level = "trace",
    skip(pod),
    fields(podname = pod.metadata.name)
)]
fn get_components(pod: &Pod) -> anyhow::Result<Vec<Component>> {
    // Read resources
    let spec = pod
        .spec
        .as_ref()
        .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?;
    let container_statuses = pod
        .status
        .as_ref()
        .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?
        .container_statuses
        .as_ref()
        .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?;
    let mut naive_cpu_time = 0;
    let mut memory_limit = 0;
    for status in container_statuses.iter() {
        let state = status
            .state
            .as_ref()
            .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?
            .terminated
            .as_ref()
            .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?;
        let started = state
            .started_at
            .as_ref()
            .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?;
        let finished = state
            .finished_at
            .as_ref()
            .ok_or(anyhow::anyhow!("Container status incomplete {}", line!()))?;
        // Try to take the resources from the status field first
        // fall back on spec field
        let resources = status
            .resources
            .as_ref()
            .or(spec
                .containers
                .iter()
                .find(|cont| cont.name == status.name)
                .ok_or(anyhow::Error::msg(format!(
                    "Container spec incomplete {}",
                    line!()
                )))?
                .resources
                .as_ref())
            .ok_or(anyhow::Error::msg(format!(
                "Container status incomplete {}",
                line!()
            )))?;
        naive_cpu_time += (finished.0 - started.0).get_seconds()
            * parse_si(
                &resources
                    .limits
                    .as_ref()
                    .ok_or(anyhow::Error::msg("No Resource limits found"))?
                    .get("cpu")
                    .ok_or(anyhow::Error::msg("No Resource limits found"))?
                    .0,
            )?;
        memory_limit += parse_si(
            &resources
                .limits
                .as_ref()
                .ok_or(anyhow::Error::msg("No Resource limits found"))?
                .get("memory")
                .ok_or(anyhow::Error::msg("No Resource limits found"))?
                .0,
        )?;
    }

    let components = vec![
        Component::new("naive_cpu_time", naive_cpu_time / 1000)?,
        Component::new("memory_limit", memory_limit / 1000)?,
    ];
    Ok(components)
}

mod kubernetes {
    /// Module for communicating with the Kubernetes API.
    use std::fmt::Debug;
    //use std::fmt;

    use anyhow::Result;
    use kube::{
        api::{Api, ListParams},
        core::{NamespaceResourceScope, Resource},
    };

    //use crate::CONFIG;

    pub struct KubeApi<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
    {
        apis: Vec<Api<K>>,
    }

    impl<K> KubeApi<K>
    where
        K: Resource<Scope = NamespaceResourceScope>,
        <K as Resource>::DynamicType: Default,
    {
        #[tracing::instrument(name = "Create K8s API wrapper", level = "debug")]
        pub async fn new(namespaces: &[&str]) -> Self {
            let config = kube::Config::infer().await.unwrap();
            let client = kube::Client::try_from(config).unwrap();
            let apis = namespaces
                .iter()
                .map(|s| Api::namespaced(client.clone(), s.to_owned()))
                .collect();
            Self {
                //client,
                apis,
            }
        }
    }

    impl<K> KubeApi<K>
    where
        K: Resource<Scope = NamespaceResourceScope> + serde::de::DeserializeOwned + Clone + Debug,
    {
        #[tracing::instrument(name = "Retrieve Pods from Kubernetes", level = "debug", skip(self))]
        pub async fn list(&self, lp: &ListParams) -> Result<ObjectIter<K>> {
            let mut lists = Vec::with_capacity(self.apis.len());
            for api in self.apis.iter() {
                lists.push(api.list(lp).await?.items);
            }
            Ok(lists.into())
        }
    }

    pub struct ObjectIter<K> {
        lists: Vec<Vec<K>>,
    }

    impl<K> ObjectIter<K> {
        pub fn len(&self) -> usize {
            self.lists.iter().map(Vec::len).sum()
        }
    }

    impl<K> Iterator for ObjectIter<K> {
        type Item = K;
        fn next(&mut self) -> Option<Self::Item> {
            while let Some(v) = self.lists.last_mut() {
                if let Some(o) = v.pop() {
                    return Some(o);
                } else {
                    self.lists.pop();
                }
            }
            None
        }
    }

    impl<K> From<Vec<Vec<K>>> for ObjectIter<K> {
        fn from(lists: Vec<Vec<K>>) -> Self {
            Self { lists }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::load_configuration;
    use auditor::domain::ValidName;
    use k8s_openapi::api::core::v1::{
        Container, ContainerState, ContainerStateTerminated, ContainerStatus, PodSpec, PodStatus,
        ResourceRequirements,
    };
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::Time;
    use std::collections::BTreeMap;

    fn testmeta() -> ObjectMeta {
        ObjectMeta {
            name: Some("testpod".to_string()),
            namespace: Some("testns".to_string()),
            uid: Some("testuuid".to_string()),
            ..ObjectMeta::default()
        }
    }

    fn testresources() -> BTreeMap<String, Quantity> {
        let mut map = BTreeMap::new();
        map.insert("cpu".to_string(), Quantity("1000".to_owned()));
        map.insert("memory".to_string(), Quantity("100".to_owned()));
        map
    }

    fn testresourcereq() -> ResourceRequirements {
        ResourceRequirements {
            claims: None,
            limits: Some(testresources()),
            requests: Some(testresources()),
        }
    }

    fn testcontainer() -> Container {
        Container {
            resources: Some(testresourcereq()),
            ..Container::default()
        }
    }

    fn testpodspec() -> PodSpec {
        PodSpec {
            containers: vec![testcontainer(), testcontainer()],
            ..PodSpec::default()
        }
    }

    fn testcontainerstateterminated() -> ContainerStateTerminated {
        ContainerStateTerminated {
            started_at: Some(Time(Timestamp::default())),
            finished_at: Some(Time(Timestamp::default())),
            ..ContainerStateTerminated::default()
        }
    }

    fn testcontainerstate() -> ContainerState {
        ContainerState {
            running: None,
            terminated: Some(testcontainerstateterminated()),
            waiting: None,
        }
    }

    fn testcontainerstatus() -> ContainerStatus {
        ContainerStatus {
            state: Some(testcontainerstate()),
            ..ContainerStatus::default()
        }
    }

    fn testpodstatus() -> PodStatus {
        PodStatus {
            phase: Some("Failed".to_owned()),
            container_statuses: Some(vec![testcontainerstatus(), testcontainerstatus()]),
            start_time: Some(Time(Timestamp::default())),
            ..PodStatus::default()
        }
    }

    fn testpod() -> Pod {
        Pod {
            metadata: testmeta(),
            spec: Some(testpodspec()),
            status: Some(testpodstatus()),
        }
    }

    #[test]
    fn parsing_success() {
        assert_eq!(parse_si("3m").unwrap(), 3);
        assert_eq!(parse_si("3").unwrap() / 1000, 3);
        assert_eq!(parse_si("3k").unwrap() / 1000, 3 * 1000_i64.pow(1));
        assert_eq!(parse_si("3Ki").unwrap() / 1000, 3 * 1024_i64.pow(1));
        assert_eq!(parse_si("3M").unwrap() / 1000, 3 * 1000_i64.pow(2));
        assert_eq!(parse_si("3Mi").unwrap() / 1000, 3 * 1024_i64.pow(2));
        assert_eq!(parse_si("3G").unwrap() / 1000, 3 * 1000_i64.pow(3));
        assert_eq!(parse_si("3Gi").unwrap() / 1000, 3 * 1024_i64.pow(3));
        assert_eq!(parse_si("3T").unwrap() / 1000, 3 * 1000_i64.pow(4));
        assert_eq!(parse_si("3Ti").unwrap() / 1000, 3 * 1024_i64.pow(4));
    }

    #[test]
    fn parsing_fail() {
        assert!(parse_si("").is_err());
        assert!(parse_si("k").is_err());
        assert!(parse_si("5Mii").is_err());
        assert!(parse_si("6⁂").is_err());
    }

    #[test]
    fn test_get_stop_time() {
        assert_eq!(get_stop_time(&testpod()).unwrap(), Timestamp::default());
    }

    #[test]
    fn test_get_components() {
        let components = get_components(&testpod()).unwrap();
        assert_eq!(components[0].name.as_ref(), "naive_cpu_time");
        assert_eq!(components[0].amount.as_ref(), &0);
        assert_eq!(components[1].name.as_ref(), "memory_limit");
        assert_eq!(components[1].amount.as_ref(), &200); // Two Containers
    }

    #[test]
    fn test_pod_to_record() {
        crate::constants::ensure_lazies();
        let _ = CONFIG.set(load_configuration("testconfig.yml").unwrap());
        let rec = pod_to_record(testpod()).unwrap().unwrap();
        assert_eq!(rec.record_id.as_ref(), "KUBE_-testns-testpod-testuuid");
        assert_eq!(rec.start_time, DateTime::<Utc>::default());
        assert_eq!(rec.stop_time.unwrap(), DateTime::<Utc>::default());
        let meta = rec.meta.unwrap();
        assert_eq!(
            meta.0.get(&KEY_PODNAME).unwrap(),
            &vec![ValidName::parse("testpod".to_owned()).unwrap()]
        );
        assert_eq!(
            meta.0.get(&KEY_NAMESPACE).unwrap(),
            &vec![ValidName::parse("testns".to_owned()).unwrap()]
        );
        assert_eq!(
            meta.0.get(&KEY_STATUS).unwrap(),
            &vec![ValidName::parse("Failed".to_owned()).unwrap()]
        );
    }
}
