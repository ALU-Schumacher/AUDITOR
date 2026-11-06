Name: auditor_htcondor_collector
Version: 0.11.2
Release: 1
Summary: AUDITOR collector for aggregating data from the HTCondor batch system
License: BSD-2-Clause-Patent

Source0: /__w/AUDITOR/AUDITOR/collectors/htcondor

Requires:       python3
Requires:       python3-pip
Requires:       systemd

%description
AUDITOR collector for aggregating data from the HTCondor batch system

%prep
%autosetup

%build

%pre
if [ "$1" -eq 1 ]; then
  getent group auditor || groupadd --system auditor
  getent passwd auditor-htcondor-collector || useradd --system --no-create-home --gid auditor --shell /sbin/nologin auditor-htcondor-collector
fi

%install
rm -rf %{buildroot}
install -d -D -m 0750 %{buildroot}/var/lib/auditor_htcondor_collector
install -d %{buildroot}/opt/auditor_htcondor_collector
install -d %{buildroot}/etc/auditor
install -d %{buildroot}/usr/libexec/%{name}
install -d %{buildroot}/usr/lib/systemd/system

install -m 0755 bootstrap-venv.sh %{buildroot}/usr/libexec/%{name}/bootstrap-venv.sh
install -m 0644 unit_files/auditor_htcondor_collector.service %{buildroot}/usr/lib/systemd/system/auditor_htcondor_collector.service
install -m 0644 configs/auditor_htcondor_collector_template.yml %{buildroot}/etc/auditor/auditor_htcondor_collector.yml

%post
if [ "$1" -ge 1 ]; then
    /usr/libexec/%{name}/bootstrap-venv.sh %{version}
    systemctl --no-reload preset auditor_htcondor_collector
fi

%preun
if [ "$1" -eq 0 ]; then
  systemctl --no-reload disable --now --no-warn auditor_htcondor_collector
fi

%postun
if [ "$1" -eq 0 ]; then
  runuser -u auditor-htcondor-collector -- rm /var/lib/auditor_htcondor_collector/*
  rmdir /var/lib/auditor_htcondor_collector/ || true
  rmdir /etc/auditor/ || true
  rm -rf /opt/auditor_htcondor_collector/venv/
  rmdir /opt/auditor_htcondor_collector || true
  userdel auditor-htcondor-collector
  groupdel auditor || true
fi

%files
%defattr(-,root,root,-)
%dir /opt/auditor_htcondor_collector
%dir /etc/auditor
%config(noreplace) /etc/auditor/auditor_htcondor_collector.yml
/usr/libexec/%{name}/bootstrap-venv.sh
/usr/lib/systemd/system/auditor_htcondor_collector.service
%dir %attr(0750 auditor-htcondor-collector auditor) /var/lib/auditor_htcondor_collector

%changelog
* Fri Oct 09 2026 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.11.2
  - Release v0.11.2 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Sep 14 2026 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.11.1
  - Release v0.11.1 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Jul 13 2026 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.11.0
  - Release v0.11.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Mar 23 2026 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.10.2
 - First version in a package
