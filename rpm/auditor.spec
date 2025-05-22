Name:           auditor
Version:        %{version_}
Release:        1%{?dist}
Summary:        AUDITOR
BuildArch:      x86_64

License:        MIT or Apache-2.0
Source0:        %{name}-%{version}.tar.gz

%define file_permissions_user root
%define file_permissions_group root

#Requires:       bash

%description
Auditor: Accounting toolchain.

%prep
%setup -q

%install
rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT/%{_bindir}
pwd
ls
cp %{name} $RPM_BUILD_ROOT/%{_bindir}
mkdir -p $RPM_BUILD_ROOT//etc/systemd/system/
cp -R /home/runner/work/AUDITOR/AUDITOR/rpm/extra_files/auditor.service $RPM_BUILD_ROOT//etc/systemd/system/auditor.service
mkdir -p $RPM_BUILD_ROOT//opt/auditor/
cp -R /home/runner/work/AUDITOR/AUDITOR/rpm/extra_files/auditor_template.yml $RPM_BUILD_ROOT//opt/auditor/auditor.yml
cp -R /home/runner/work/AUDITOR/AUDITOR/migrations/20220322080444_create_accounting_table.sql $RPM_BUILD_ROOT//opt/auditor/20220322080444_create_accounting_table.sql
cp -R /home/runner/work/AUDITOR/AUDITOR/migrations/20240503141800_convert_meta_component_to_jsonb.sql $RPM_BUILD_ROOT//opt/auditor/20240503141800_convert_meta_component_to_jsonb.sql

%clean
rm -rf $RPM_BUILD_ROOT

%files
%defattr(-,%{file_permissions_user},%{file_permissions_group},-)
%{_bindir}/%{name}
//etc/systemd/system/auditor.service
%config(noreplace) //opt/auditor/auditor.yml
//opt/auditor/20220322080444_create_accounting_table.sql
//opt/auditor/20240503141800_convert_meta_component_to_jsonb.sql

%changelog
* Fri May 23 2025 Dirk Sammel <dirk.sammel@physik.uni-freiburg.de> - 0.9.4
  - Release v0.9.4 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Wed May 14 2025 Dirk Sammel <dirk.sammel@physik.uni-freiburg.de> - 0.9.3
  - Release v0.9.3 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Thu Apr 10 2025 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.9.2
  - Release v0.9.2 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Mar 31 2025 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.9.1
  - Release v0.9.1 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Thu Mar 27 2025 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.9.0
  - Release v0.9.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Mar 03 2025 Dirk Sammel <dirk.sammel@physik.uni-freiburg.de> - 0.8.0
  - Release v0.8.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Thu Feb 27 2025 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.7.1
  - Release v0.7.1 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Jan 27 2025 Dirk Sammel <dirk.sammel@physik.uni-freiburg.de> - 0.7.0
  - Release v0.7.0 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Wed Oct 30 2024 Dirk Sammel <dirk.sammel@physik.uni-freiburg.de> - 0.6.3
  - Release v0.6.3 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
* Mon Jul 29 2024 Raghuvar Vijayakumar <raghuvar.vijayakumar@physik.uni-freiburg.de> - 0.6.2
  - Release v0.6.2 - see https://github.com/ALU-Schumacher/AUDITOR/blob/main/CHANGELOG.md for changes
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
