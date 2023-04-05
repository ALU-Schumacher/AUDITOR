# HTCondor-Collector
- [HTCondor-Collector](#htcondor-collector)
  - [Configuration](#configuration)
    - [`entry`](#entry)
  - [Example config](#example-config)

The collector relies on `condor_history` to retrieve the information about the jobs.
The collector runs periodically, creating [records](https://alu-schumacher.github.io/AUDITOR/pyauditor/api.html#pyauditor.Record) and committing them to the AUDITOR-instance using [pyauditor](https://alu-schumacher.github.io/AUDITOR/pyauditor/).

The collector is run as follows:

```bash
python -m collectors.htcondor -c CONFIG_FILE
```

`-c/--config CONFIG_FILE` is required to be set and of the form as stated below.
Furter, optional arguments are
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

## Configuration
The collector is configured using a yaml-file. Configuration parameters are as follows:

| Parameter       | Description                                                                                                                                                                                                                                             |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `addr`          | Address of the AUDITOR-instance.                                                                                                                                                                                                                        |
| `port`          | Port of the AUDITOR-instance.                                                                                                                                                                                                                           |
| `state_db`      | Path to the sqlite-database used for persistent storage of the job ids last processed by the collector.                                                                                                                                                 |
| `record_prefix` | Prefix used for all records put into the AUDITOR-database.                                                                                                                                                                                              |
| `interval`      | Interval in seconds beteween runs of the collector.                                                                                                                                                                                                     |
| `pool`          | The `-pool` argument used for the invocation of `condor_history`.                                                                                                                                                                                       |
| `schedd_names`  | List of the schedulers used for the `-name` argument of the invocation of `condor_history`.                                                                                                                                                             |
| `job_status`    | List of job statuses considered. See [HTCondor magic numbers](https://htcondor-wiki.cs.wisc.edu/index.cgi/wiki?p=MagicNumbers).                                                                                                                         |
| `meta`          | Map key/value pairs put in the records meta field. The key is used as the key in the records meta-variables, the values are [`entry`](#entry)s.<br>If multiple [`entry`](#entry)s are given for specified name, the values are appended to a list. A special case is `site`, which is a list of [`entry`](#entry)s, but only the first match is used. |
| `components`    | List of components ([`entry`](#entry)s) put in the [records component](https://alu-schumacher.github.io/AUDITOR/pyauditor/api.html#pyauditor.Component)s. Each component can contain a list of [score](https://alu-schumacher.github.io/AUDITOR/pyauditor/api.html#pyauditor.Score)s ([`entry`](#entry)s).                                                                                                                                    |

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

## Example config
```yaml
addr: localhost
port: 8000
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