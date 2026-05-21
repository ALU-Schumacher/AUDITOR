Name: auditor_utilization_plugin
Version: 0.10.2
Release: 1
Summary: AUDITOR plugin to create cluster utilization report
License: BSD-2-Clause-Patent

Source0: /__w/AUDITOR/AUDITOR/plugins/utilization_tool

Requires:       python3
Requires:       python3-pip
Requires:       python3-virtualenv
Requires:       systemd

%description
AUDITOR plugin to create cluster utilization report

%prep
%autosetup

%build

%pre
if [ "$1" -eq 1 ]; then
  getent group auditor || groupadd --system auditor
  getent passwd auditor-utilization-plugin || useradd --system --no-create-home --gid auditor --shell /sbin/nologin auditor-utilization-plugin
fi

%install
rm -rf %{buildroot}
install -d -D -m 0750 %{buildroot}/var/lib/auditor_utilization_plugin
install -d %{buildroot}/opt/auditor_utilization_plugin
install -d %{buildroot}/etc/auditor
install -d %{buildroot}/usr/libexec/%{name}
install -d %{buildroot}/usr/lib/systemd/system

install -m 0755 bootstrap-venv.sh %{buildroot}/usr/libexec/%{name}/bootstrap-venv.sh
install -m 0644 unit_files/auditor_utilization_plugin.service %{buildroot}/usr/lib/systemd/system/auditor_utilization_plugin.service
install -m 0644 configs/auditor_utilization_template.yml %{buildroot}/etc/auditor/auditor_utilization_plugin.yml

%post
if [ "$1" -ge 1 ]; then
    /usr/libexec/%{name}/bootstrap-venv.sh %{version}
    systemctl --no-reload preset auditor_utilization_plugin
fi

%preun
if [ "$1" -eq 0 ]; then
  systemctl --no-reload disable --now --no-warn auditor_utilization_plugin
fi

%postun
if [ "$1" -eq 0 ]; then
  runuser -u auditor-utilization-plugin -- rm /var/lib/auditor_utilization_plugin/*
  rmdir /var/lib/auditor_utilization_plugin/ || true
  rmdir /etc/auditor/ || true
  rm -rf /opt/auditor_utilization_plugin/venv/
  rmdir /opt/auditor_utilization_plugin || true
  userdel auditor-utilization-plugin
  groupdel auditor || true
fi

%files
%defattr(-,root,root,-)
%dir /opt/auditor_utilization_plugin
%dir /etc/auditor
%config(noreplace) /etc/auditor/auditor_utilization_plugin.yml
/usr/libexec/%{name}/bootstrap-venv.sh
/usr/lib/systemd/system/auditor_utilization_plugin.service
%dir %attr(0750 auditor-utilization-plugin auditor) /var/lib/auditor_utilization_plugin

%changelog
* Mon Mar 23 2026 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.10.2
 - First version in a package

