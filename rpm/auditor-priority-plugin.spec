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
* Thu Jul 25 2024 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.6.1
  - Release v0.6.1 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Tue Apr 23 2024 Dirk Sammel <dirk.sammel@physik.uni-freiburg.de> - 0.5.0
  - Release v0.5.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Wed Jan 31 2024 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.4.0
  - Release v0.4.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Fri Nov 24 2023 Benjamin Rottler <benjamin.rottler@physik.uni-freiburg.de> - 0.3.1
  - Release v0.3.1 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Fri Nov 17 2023 Benjamin Rottler <benjamin.rottler@physik.uni-freiburg.de> - 0.3.0
  - Release v0.3.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Thu Sep 21 2023 Benjamin Rottler <benjamin.rottler@physik.uni-freiburg.de> - 0.2.0
  - Release v0.2.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Wed Jun 15 2022 Stefan Kroboth <stefan.kroboth@gmail.com> - 0.1.0
  - First version in a package
