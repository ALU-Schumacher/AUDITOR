# AUDITOR-UTILIZATION-plugin

Plugin for AUDITOR to generate utilization report for cluster utilization and CO2 footprint

## Monthly reports with systemd

The RPM installs `auditor_utilization_plugin.timer`, which starts the service at
03:00 on the first day of each month, in the server's local time zone. The service
runs with `--oneshot`, reports the previous calendar month and exits. The
`utilization.interval` setting is not used by this execution mode.

After adjusting `/etc/auditor/auditor_utilization_plugin.yml`, enable the timer:

```sh
systemctl daemon-reload
systemctl enable --now auditor_utilization_plugin.timer
systemctl list-timers --all auditor_utilization_plugin.timer
```

When upgrading from the continuously running service, first disable and stop the
old service before enabling the timer:

```sh
systemctl disable --now auditor_utilization_plugin.service
```

Review any service drop-ins that override `Type`, `ExecStart`, `Restart` or
`WorkingDirectory`, as they take precedence over the packaged service. The timer
is subject to the host's systemd preset policy on a fresh RPM installation;
upgrades do not automatically migrate an already enabled service to the timer.

The timer is persistent: after downtime, one missed activation is caught up when
the timer starts again. This generates the previous calendar month's report at
the time of execution; it does not backfill every missed month. If a report is
still running at the next activation, systemd does not start another copy of the
same service.

To generate a report immediately, run:

```sh
systemctl start auditor_utilization_plugin.service
```

The service normally appears inactive between reports; inspect the timer for
the next scheduled run. Its working directory is
`/var/lib/auditor_utilization_plugin`, owned by the service account, so relative
log and output paths resolve there. Use absolute paths for inputs such as
certificates. Application files under `/opt/auditor_utilization_plugin` remain
root-owned.

## License

Licensed under

 - BSD-2-Clause Plus Patent License ([LICENSE](https://github.com/ALU-Schumacher/AUDITOR/tree/main/plugins/apel/LICENSE))

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you shall be licensed as above, without any additional terms or conditions.
