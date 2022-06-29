Name:           auditor-priority-plugin
Version:        %{version_}
Release:        1%{?dist}
Summary:        Priority plugin for AUDITOR
BuildArch:      x86_64

License:        MIT or Apache-2.0
Source0:        %{name}-%{version}.tar.gz

#Requires:       bash

%description
Priority plugin for Auditor

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
* Wed Jun  15 2022 Stefan Kroboth <stefan.kroboth@gmail.com> - 0.1.0
- First version in a package 
