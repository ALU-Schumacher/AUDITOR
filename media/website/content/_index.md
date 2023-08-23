+++
title = "Auditor"
sort_by = "weight"
+++

# Auditor

Auditor stands for **A**cco**u**nting **D**ata Handl**i**ng **T**oolbox For **O**pportunistic **R**esources.
Auditor ingests accounting data provided by so-called *collectors*, stores it and provides it to the outside to so-called *plugins*.

It comes with a well-defined REST API which allows for the implementation of application-specific collectors and plugins. This makes it well suited for a wide range of use cases.

<p align="center">
  <img
    width="700"
    src="auditor_overview.png"
  />
</p>

Overview of the AUDITOR ecosystem. AUDITOR accepts records from collectors, stores them in a PostgreSQL
database and o ers these records to plugins which take an action based on the records.

## Features


# Running Auditor

Auditor can be run by compiling the source from the repository or by running a pre-built docker container.
Both methods require that the PostgreSQL database is installed migrated beforehand.

## Migrating the database

Migrating the database requires cloning the Auditor repository and installing `cargo` and `sqlx`.
A prerequisite is a working Rust setup, which can be installed either via your distributions package manager or via the following command:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Now `sqlx` can be installed via `cargo`:

```bash
cargo install --version=0.7.1 sqlx-cli --no-default-features --features postgres,rustls,sqlite
```

Clone the repository and `cd` into the directory.

```bash
git clone git@github.com:ALU-Schumacher/AUDITOR.git
cd AUDITOR
```

To migrate the database, run the following from the root directory of the repo:

```bash
# Adapt thesee variables to your setup
DB_USER="postgres"
DB_PASSWORD="password"
DB_NAME="auditor"
DB_HOST="localhost"
DB_PORT="5432"

export DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
sqlx database create
sqlx migrate run
```

This assumes that the PostgreSQL database is already installed.

## Using Docker

The easiest way to run Auditor is via a Docker container from [Docker Hub](https://hub.docker.com/repository/docker/aluschumacher/auditor) or [Github Container Registry](https://github.com/ALU-Schumacher/AUDITOR/pkgs/container/auditor).
Auditor requires a properly configured PostgreSQL database.
After installing PostgreSQL, the database needs to be migrated with `sqlx`.

AUDITORs configuration can be adapted with environment variables.

| Variable                          | Description                                               |
| --------                          | -----------                                               |
| `AUDITOR_APPLICATION__ADDR`       | Address to bind to (default `0.0.0.0`)                    |
| `AUDITOR_APPLICATION__PORT`       | Port to bind to (default `8000`)                          |
| `AUDITOR_DATABASE__HOST`          | Host address of PostgreSQL database (default `localhost`) |
| `AUDITOR_DATABASE__PORT`          | Port of PostgreSQL database (default `5432`)              |
| `AUDITOR_DATABASE__USERNAME`      | PostgreSQL database username (default `postgres`)         |
| `AUDITOR_DATABASE__PASSWORD`      | PostgreSQL database password (default `password`)         |
| `AUDITOR_DATABASE__REQUIRE_SSL`   | Whether or not to use SSL (default `true`)                |

Use `docker run` to execute Auditor:

```bash
docker run aluschumacher/auditor:main
```

The configuration parameters can be set by passing environment variables via `-e`:

```bash
docker run -e AUDITOR_APPLICATION__ADDR=localhost -e AUDITOR_DATABASE__REQUIRE_SSL=false aluschumacher/auditor:main
```

## Configuration files

Besides environment variables, a YAML configuration file can be used:

```yaml
application:
  addr: 0.0.0.0
  port: 8000
database:
  host: "localhost"
  port: 5432
  username: "postgres"
  password: "password"
  database_name: "auditor"
  require_ssl: false
metrics:
  database:
    frequency: 30
    metrics:
      - RecordCount
      - RecordCountPerSite
      - RecordCountPerGroup
      - RecordCountPerUser
```

This configuration file can be passed to Auditor and will overwrite the default configuration.

## Metrics exporter for Prometheus

Metrics for Prometheus are exposed via the `/metrics` endpoint.
By default HTTP metrics are exported.
In addition, database metrics are exported as well (optional).
These include the current number of records in the database, as a well as the number of records per site, group and user.
Database metrics export can be configured in the configuration:

```yaml
metrics:
  database:
    # How often these values are computed (default: every 30 seconds)
    frequency: 30
    # Type of metrics to export (default: None)
    metrics:
      - RecordCount
      - RecordCountPerSite
      - RecordCountPerGroup
      - RecordCountPerUser
```

How often the database metrics are computed is defined by the `frequency` configuration variable.
Note that computing the database metrics is a potentially expensive operation.
Therefore it is advised to monitor the performance of Auditor when working with databases with a large number of records.
The frequency setting should be somewhat in accordance with the Prometheus scraping interval.


# Compiling Auditor

Alternatively, Auditor can be compiled and run directly.
Instructions for compiling Auditor from source can be found in the [development](development/#compiling-auditor-from-source) documentation.

# Packages

RPMs are provided for each release on the [Github release page](https://github.com/ALU-Schumacher/AUDITOR/releases).

# Collectors

Collectors are used to collect data from various sources.
See below for all currently available collectors.

## SLURM Collector

TODO

## SLURM Epilog Collector

The Slurm epilog collector can installed from the provided RPM or can be built with this command:

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl --bin auditor-slurm-epilog-collector
```

The resulting binary can be found in `target/x86_64-unknown-linux-musl/release/auditor-slurm-epilog-collector` and is ideally placed on the Slurm head node.

Add this to your epilog shell script (on the slurm head node):

```bash
#!/bin/sh

# Divert stdout and sterr. Make sure the slurm user has write access to both locations.
# Ideally there is also log rotation in place for those logs.
exec >> /epilog_logs/epilog.log
exec 2>> /epilog_logs/epilog.log

/absolute/path/to/auditor-slurm-epilog-collector /absolute/path/to/auditor-slurm-epilog-collector-config.yaml
```

This will read the `$SLURM_JOB_ID` environment variable, which is only available in the context of a SLURM epilog script.

Internally, `scontrol` is called to obtain the necessary information of the job.

If not all jobs are of relevance, filtering should be done in the epilog script such that the collector is only executed for relevant jobs.
This avoids unnecessary and potentially expensive calls to `scontrol`.
Slurm provides a number of environment variables in the context of an epilog script which are listed in the [Slurm documentation](https://slurm.schedmd.com/prolog_epilog.html).

Example:

```bash
#!/bin/sh

# Only execute collector for jobs running on `some_partition`
if [ "$SLURM_JOB_PARTITION" == "some_partition" ]; then
	LOG_FILE=/path/to/epilog.log
	exec >> $LOG_FILE
	exec 2>> $LOG_FILE

	/absolute/path/to/auditor-slurm-epilog-collector /absolute/path/to/auditor-slurm-epilog-collector-config.yaml
fi
```

### Example configurations

The following configuration shows how to set the Auditor host address and port.
The `record_prefix` will be used to prefix the Slurm job id in the record identifier (in this case it will be `slurm-JOBID`).
The `site_name` is the `site_id` which will be attached to every record.
`components` defines how to extract accountable information from the call to `scontrol` and attaches `score`s to it.
In the context of `components`, `name` indicates how this component will be identified in the final record and `key` indicates the `key` which is to be extracted from the `scontrol` output.
`scores` are optional.

```yaml
addr: "auditor_host_addr"
port: 8000
record_prefix: "slurm"
site_id: "site_name"
components:
  - name: "Cores"
    key: "NumCPUs"
    scores:
      - name: "HEPSPEC"
        value: 1.0
  - name: "Memory"
    key: "Mem"
```

Extraction of components as well as adding of scores can be done conditionally, as shown in the following example configuration.
The matching is performed on values associated with certain keys in the `scontrol` output.
Regex is accepted.

```yaml
addr: "auditor_host_address"
port: 8000
record_prefix: "slurm"
site_id: "site_name"
components:
  - name: "Cores"
    key: "NumCPUs"
    scores:
      # If it the job is running on partition `part1`, then use HEPSPEC value 1.1
      - name: "HEPSPEC"
        value: 1.1
        only_if:
          key: "Partition"
          matches: "^part1$"
      # If it the job is running on partition `part2`, then use HEPSPEC value 1.2
      - name: "HEPSPEC"
        value: 1.2
        only_if:
          key: "Partition"
          matches: "^part2$"
  - name: "Memory"
    key: "Mem"
    only_if:
      key: "Partition"
      matches: "^part2$"
```

## HTCondor Collector

The collector relies on `condor_history` to retrieve the information about the jobs.
The collector runs periodically, creating [records](https://alu-schumacher.github.io/AUDITOR/pyauditor/api.html#pyauditor.Record) and committing them to the AUDITOR-instance using [pyauditor](https://alu-schumacher.github.io/AUDITOR/pyauditor/).

The collector is run as follows:

```bash
python -m collectors.htcondor -c CONFIG_FILE
```

`-c/--config CONFIG_FILE` is required to be set and of the form as stated below.
Further, optional arguments are
```
-h, --help            show this help message and exit
-c CONFIG_FILE, --config CONFIG_FILE
                      Path to config file.
-j <CLUSTERID>[.<PROCID>], --job-id <CLUSTERID>[.<PROCID>]
                      ID of the job, condor_history to invoke with.
-n SCHEDD, --schedd-names SCHEDD
                      Name of the schedd, condor_history to invoke with.
-l {DEBUG,INFO,WARNING,ERROR,CRITICAL}, --log-level {DEBUG,INFO,WARNING,ERROR,CRITICAL}
                      Log level. Defaults to INFO.
-f LOG_FILE, --log-file LOG_FILE
                      Log file. Defaults to stdout.
-i INTERVAL, --interval INTERVAL
                      Interval in seconds between queries. Defaults to 900.
-1, --one-shot        Run once and exit.
```
Command line arguments override the values set in the config file.

### Configuration
The collector is configured using a yaml-file. Configuration parameters are as follows:

| Parameter       | Description                                                                                                                                                                                                                                                                                                                                           |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `state_db`      | Path to the sqlite-database used for persistent storage of the job ids last processed by the collector.                                                                                                                                                                                                                                               |
| `record_prefix` | Prefix used for all records put into the AUDITOR-database.                                                                                                                                                                                                                                                                                            |
| `interval`      | Interval in seconds between runs of the collector.                                                                                                                                                                                                                                                                                                    |
| `pool`          | The `-pool` argument used for the invocation of `condor_history`.                                                                                                                                                                                                                                                                                     |
| `schedd_names`  | List of the schedulers used for the `-name` argument of the invocation of `condor_history`.                                                                                                                                                                                                                                                           |
| `job_status`    | List of job statuses considered. See [HTCondor magic numbers](https://htcondor-wiki.cs.wisc.edu/index.cgi/wiki?p=MagicNumbers).                                                                                                                                                                                                                       |
| `meta`          | Map key/value pairs put in the records meta field. The key is used as the key in the records meta-variables, the values are [`entry`](#entry)s.<br>If multiple [`entry`](#entry)s are given for specified name, the values are appended to a list. A special case is `site`, which is a list of [`entry`](#entry)s, but only the first match is used. |
| `components`    | List of components ([`entry`](#entry)s) put in the [records component](https://alu-schumacher.github.io/AUDITOR/pyauditor/api.html#pyauditor.Component)s. Each component can contain a list of [score](https://alu-schumacher.github.io/AUDITOR/pyauditor/api.html#pyauditor.Score)s ([`entry`](#entry)s).                                            |

The following parameters are optional:

| Parameter | Default            | Description                                                                     |
| --------- | ------------------ | ------------------------------------------------------------------------------- |
| `addr`    | `http://127.0.0.1` | Address of the AUDITOR-instance. If this is set, `port` must also be specified. |
| `port`    | `8080`             | Port of the AUDITOR-instance. If this is set, `addr` must also be specified.    |
| `timeout` | `30`               | Timeout in seconds for the connection to the AUDITOR-instance.                  |

### `entry`
An `entry` describes how to get the value for a meta-var or component from the job.
Unlike meta-variables, components contain a `name`-field, which is used as the name of the component.
If the entry has a `key`-field, the value is taken from the corresponding ClassAd.
Else, if the entry has a `factor`-field, this factor is used as the value.
Else, if the entry has a `name`-field, this name is used as the value (this is used for the `site`-meta-var).
Else, the value is not set.

If the entry has a `matches`-field, the value is matched against the regex given in `matches`.
In case the regex contains a group, the value is set to the (first) matching group, else the `name`-field is used.

If the entry contains an `only_if`-field, the value is only returned if the value of the ClassAd in `only_if.key`  matches the regex given in `only_if.matches`.

See below for an example config and the use of such `entry`s.

### Example config
```yaml
addr: localhost
port: 8000
timeout: 10
state_db: htcondor_history_state.db
record_prefix: htcondor
interval: 900 # 15 minutes
pool: htcondor.example.com
schedd_names:
  - schedd1.example.com
  - schedd2.example.com
job_status: # See https://htcondor-wiki.cs.wisc.edu/index.cgi/wiki?p=MagicNumbers
  - 3 # Removed
  - 4 # Completed

meta:
  user:
    key: Owner
    matches: ^(.+)$
  group:
    key: VoName
    matches: ^(.+)$
  submithost:
    key: "GlobalJobId"
    matches: ^(.*)#\d+.\d+#\d+$  # As this regex contains a group, the value for 'submithost' is set to the matching group.

  # For `site` the first match is used.
  site:
    - name: "site1"  # This entry
      key: "LastRemoteHost"
      matches: ^slot.+@site1-.+$
    - key: "LastRemoteHost"
      matches: ^slot.+@(site2)-.+$  # This regex contains a group, the value for 'site' is set to the matching group ("site2").
    - name: "UNDEF"  # If no match is found, site is set to "UNDEF"

components:
  - name: "Cores"
    key: "CpusProvisioned"
    scores:
      - name: "HEPSPEC"
        key: "MachineAttrApelSpecs0"
        matches: HEPSPEC\D+(\d+(\.\d+)?)  # This regex matches the value of HEPSPEC in the corresponding ClassAd
        only_if:
          key: "LastRemoteHost"
          matches: ^slot.+@(?:site1)-.{10}@.+$  # This score is only attributed to the component on site1
      - name: "HEPscore23"
        key: "MachineAttrApelSpecs0"
        matches: HEPscore23\D+(\d+(\.\d+)?)
        only_if:
          key: "LastRemoteHost"
          matches: ^slot.+@(?:site1)-.{10}@.+$
  - name: "Memory"
    key: "MemoryProvisioned"
  - name: "UserCPU"
    key: "RemoteUserCpu"
```

# Plugins

Plugins are used to retrieve data from Auditor for further processing.
See below for all currently available collectors.

## APEL Plugin

The APEL plugin creates job summary records and sends them to APEL.
The following fields need to be present in the config file:

| Parameter             | Description                                                                                                                                                               |
|-----------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `log_level`           | Can be set to `DEBUG`, `INFO`, `WARNING`, `ERROR`, or `CRITICAL` (with decreasing verbosity).                                                                             |
| `time_db_path`        | Path of the `time.db`. The database should be located at a persistent path and stores the end time of the latest reported job, and the time of the latest report to APEL. |
| `report_interval`     | Time in seconds between reports to APEL.                                                                                                                                  |
| `publish_since`       | Date and time (UTC) after which jobs will be published. Only relevant for first run when no `time.db` is present yet.                                                     |
| `sites_to_report`     | List of sites that will be reported. Uses the site name as stored in the AUDITOR records.                                                                                 |
| `site_name_mapping`   | Maps the site name as stored in the AUDITOR record to the name of the site in the GOCDB.                                                                                  |
| `default_submit_host` | Default submit host if this information is missing in the AUDITOR record.                                                                                                 |
| `infrastructure_type` | Origin of the job, can be set to `grid` or `local`.                                                                                                                       |
| `benchmark_type`      | Name of the benchmark that will be reported to APEL.                                                                                                                      |
| `auditor_ip`          | IP of the AUDITOR instance.                                                                                                                                               |
| `auditor_port`        | Port of the AUDITOR instance.                                                                                                                                             |
| `auditor_timeout`     | Time in seconds after which the connection to the AUDITOR instance times out.                                                                                             |
| `benchmark_name`      | Name of the `benchmark` field in the AUDITOR records.                                                                                                                     |
| `cores_name`          | Name of the `cores` field in the AUDITOR records.                                                                                                                         |
| `cpu_time_name`       | Name of the field that stores the total CPU time in the AUDITOR records.                                                                                                  |
| `cpu_time_unit`       | Unit of total CPU time in the AUDITOR records, can be `seconds` or `milliseconds`.                                                                                        |
| `nnodes_name`         | Name of the field that stores the number of nodes in the AUDITOR records.                                                                                                 |
| `meta_key_site`       | Name of the field that stores the name of the site in the AUDITOR records.                                                                                                |
| `meta_key_submithost` | Name of the field that stores the submithost in the AUDITOR records.                                                                                                      |
| `meta_key_voms`       | Name of the field that stores the VOMS information in the AUDITOR records.                                                                                                |
| `meta_key_user`       | Name of the field that stores the GlobalUserName in the AUDITOR records.                                                                                                  |
| `auth_url`            | URL from which the APEL authentication token is received.                                                                                                                 |
| `ams_url`             | URL to which the reports are sent.                                                                                                                                        |
| `client_cert`         | Path of the host certificate.                                                                                                                                             |
| `client_key`          | Path of the host key.                                                                                                                                                     |
| `ca_path`             | Path of the local certificate folder.                                                                                                                                     |
| `verify_ca`           | Controls the verification of the certificate of the APEL server. Can be set to `True` or `False` (the latter might be necessary for local test setups).                   |


Example config:

```
[logging]
log_level = INFO

[paths]
time_db_path = /etc/auditor_apel_plugin/time.db

[intervals]
report_interval = 86400

[site]
publish_since = 2023-01-01 13:37:42+00:00
sites_to_report = ["site1", "site2"]
site_name_mapping = {"site1": "SITE-2-GOCDB", "site2": "SITE-2-GOCDB"}
default_submit_host = gsiftp://accounting.grid_site.de:1337/jobs
infrastructure_type = grid
benchmark_type = hepscore23

[auditor]
auditor_ip = 127.0.0.1
auditor_port = 3333
auditor_timeout = 60
benchmark_name = hepscore23
cores_name = Cores
cpu_time_name = TotalCPU
cpu_time_unit = milliseconds
nnodes_name = NNodes
meta_key_site = site_id
meta_key_submithost = headnode
meta_key_voms = voms
meta_key_username = subject

[authentication]
auth_url = https://msg.argo.grnet.gr:8443/v1/service-types/ams/hosts/msg.argo.grnet.gr:authx509
ams_url = https://msg-devel.argo.grnet.gr:443/v1/projects/accounting/topics/gLite-APEL:publish?key=
client_cert = /etc/grid-security/hostcert.pem
client_key = /etc/grid-security/hostkey.pem
ca_path = /etc/grid-security/certificates
verify_ca = True
```

## Priority Plugin

The priority plugin takes the resources provided by multiple groups and computes a priority for each of these groups based on who many resources wer provided.
This allows one to transfer provided resources on one system to priorities in another system.
The computed priorities are set via shelling out, and the executed commands can be defined as needed.

The priority plugin is available as RPM or can be built with the command:

```bash
RUSTFLAGS='-C link-arg=-s' cargo build --release --target x86_64-unknown-linux-musl --bin auditor-priority-plugin
```

The resulting binary can be found in `target/x86_64-unknown-linux-musl/release/auditor-priority-plugin` and is ideally placed on a node where the priorities should be set.

One run of the priority plugin will update the priorities once.
In general, the priority plugin should be run on a regular basis (for instance as a cron job), depending on how often the update should be performed.

A typical configuration for the SLURM batch system may look like this:

```yaml
addr: "auditor_host_address"
port: 8000
timeout: 30 # in seconds
duration: 1209600 # in seconds
components:
  NumCPUs: "HEPSPEC"
group_mapping:
  group1:
    - "part1"
  group2:
    - "part2"
  group3:
    - "part3"
command:
  - "/usr/bin/scontrol update PartitionName={1} PriorityJobFactor={priority}"
min_priority: 1
max_priority: 65335
computation_mode: ScaledBySum
```

The resources used for calculating the priorities can be configured via the `components` field.
It defines which components to extract from the `components` field of the record (`NumCPUs` in this example), as well as the corresponding score (`HEPSPEC` in this example).
Multiple components can be extracted.
The configured components and scores must be part of the records.
The resources of each component will be multiplied by the corresponding score and the resulting provided resource per group is the sum of all these.
The records considered in the computation can be limited to all records which finished in the past X seconds via the `duration` field (in seconds).
Omitting this field takes all records in the database into account.
Via the `group_mapping` field, it is possible to attach certain additional information to the individual groups which are to be considered in the calculation.
In the example configuration above are three groups `group{1,2,3}`, where each has a corresponding partition `part{1,2.3}`.
These mappings can be accessed when constructing the `command`s which will be executed after computing the priorities by using `{N}` where `N` corresponds to number of the element in the list of the `group_mapping`.
For instance, for `group1`, the string `{1}` indicates `part1` while for `group2` the same string `{1}` indicates `part2`.
The `command` field in the configuration illustrates the usage of these mappings.
This allows one to adapt the command for the various groups involved.
In the `command` field one can also see a string `{priority}`, which will be replaced by the computed priority for the group.
Another special string, `{resources}` is available, which is replaced by the computed provided resource per group.
The command is executed for each group separately and multiple commands can be provided.

### Priority computation modes

As stated above, the priorities are computed from the provided resources of each group.
However, the computed resources and the priorities are in different units and span different ranges.
Therefore a mapping between resources and priorities needs to be in place.
This plugin offers two `computation_modes`: `FullSpread` and `ScaledBySum`.
Via `min_priority` and `max_priority`, lower and upper limits on the computed priority are set.

* `FullSpread`:  This mode will spread the resources on the full range given by `min_priority` and `max_priority`, such that the group with the least provided resources will be assigned a priority equal to `min_priority` and the group with the most provided resources will be assigned a priority equal to `max_priority`. All other groups are distributed inside that range according to their provided resources. This creates maximum spread of the priorities. A disadvantage of this approach is that the computed priorites of two consecutive runs can be substantially different, leading to large jumps in priorities.
* `ScaledBySum`: Computes the priorities such that `max_priority` is equal to the sum of all provide resources plus `min_priority`. This leads to a smoother change of priorities over multiple runs of the plugin. The maximum priority can only be reached by a group if all other groups provide no resources.

# Auditor Clients

To facilitate the development of collectors and plugins, client libraries for Rust and Python are offered which handle the interaction with the Auditor server.
For details please consult the respective documentation pages for [the Rust client](https://docs.rs/auditor/) and [the Python client](https://ALU-Schumacher.github.io/AUDITOR/pyauditor/).

# License

Licensed under either of

 - Apache License, Version 2.0, ([LICENSE-APACHE](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT License ([LICENSE-MIT](https://github.com/ALU-Schumacher/AUDITOR/blob/main/LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

