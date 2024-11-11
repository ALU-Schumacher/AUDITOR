Name:           auditor-kubernetes-collector
Version:        %{version_}
Release:        1%{?dist}
Summary:        Kubernetes collector for AUDITOR
BuildArch:      x86_64

License:        MIT or Apache-2.0
Source0:        %{name}-%{version}.tar.gz

#Requires:       bash

%description
Kubernetes collector for Auditor

%prep
%setup -q

%install
rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/%{_bindir}
pwd
ls
cp %{name} $RPM_BUILD_ROOT/%{_bindir}

%clean
rm -rf $RPM_BUILD_ROOT

%files
%{_bindir}/%{name}

%changelog
* Mon Oct 28 2024 Raphael Kleinemühl <kleinemuehl@uni-wuppertal.de>
  - Release 0.6.0 - First version
