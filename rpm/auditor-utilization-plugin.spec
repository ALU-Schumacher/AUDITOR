Name:           auditor-utilization-plugin
Version:        0.10.1
Release:        1%{?dist}
Summary:        Utilization plugin for AUDITOR
BuildArch:      x86_64

License:        MIT or Apache-2.0

%description
Utilization plugin for Auditor

%global unitdir /usr/lib/systemd/system
%global confdir %{_sysconfdir}/auditor
%global statedir %{_localstatedir}/lib/%{name}
%global user %{name}

%pre
# On install
if [ "$1" -eq 1 ]; then
  getent group auditor || groupadd --system auditor
  getent passwd %{user} || useradd --system --no-create-home --gid auditor --shell /sbin/nologin %{user}
fi

%post
# On install
if [ "$1" -eq 1 ]; then
  systemctl --no-reload preset %{name}
fi
# On update
if [ "$1" -eq 2 ]; then
  systemctl set-property %{name} Markers=+needs-restart
fi

%preun
# On uninstall
if [ "$1" -eq 0 ]; then
  systemctl --no-reload disable --now --no-warn %{name}
fi

%postun
# On uninstall
if [ "$1" -eq 0 ]; then
  # Remove files and empty folders
  runuser -u %{user} -- rm -rf %{statedir}/*
  rmdir %{statedir} || true
  rmdir %{confdir} || true
  userdel %{user}
  groupdel auditor || true
fi

%install
install -d -D -m 0750 $RPM_BUILD_ROOT/%{statedir}
install -p -D -m 0755 -t $RPM_BUILD_ROOT/%{_bindir} %{name}
install -p -D -m 0644 -t $RPM_BUILD_ROOT/%{unitdir} %{name}.service
install -p -D -m 0644 -t $RPM_BUILD_ROOT/%{confdir} %{name}.yml
pwd
ls

%clean
rm -rf $RPM_BUILD_ROOT

%files
%dir %attr(0750,%{user},auditor) %{statedir}
%{_bindir}/%{name}
%{unitdir}/%{name}.service
%config(noreplace) %{confdir}/%{name}.yml

%changelog
* Thu Oct 30 2025 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.10.1
  - First version in a package
