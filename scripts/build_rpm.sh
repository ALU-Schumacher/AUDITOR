#!/usr/bin/env bash
set -x
set -eo pipefail

BINARY=${BINARY:="auditor-slurm-epilog-collector"}
CRATE_VERSION=$(cargo get package.version --entry auditor)

mkdir -p target/rpm/${BINARY}/rpmbuild
mkdir -p target/rpm/${BINARY}/rpmbuild/BUILD
mkdir -p target/rpm/${BINARY}/rpmbuild/RPMS
mkdir -p target/rpm/${BINARY}/rpmbuild/SOURCES
mkdir -p target/rpm/${BINARY}/rpmbuild/SPECS
mkdir -p target/rpm/${BINARY}/rpmbuild/SRPMS

# Copy binary
cp target/x86_64-unknown-linux-musl/release/${BINARY} target/rpm/${BINARY}/rpmbuild/

# Copy unit file if it exists
if [[ -f rpm/unit_files/${BINARY}.service ]]; then
    cp rpm/unit_files/${BINARY}.service target/rpm/${BINARY}/rpmbuild/
fi

# Copy config file if it exists
if [[ -f rpm/config_files/${BINARY}.yml ]]; then
    cp rpm/config_files/${BINARY}.yml target/rpm/${BINARY}/rpmbuild/
fi

# Copy migration sql files
cp migrations/*.sql target/rpm/${BINARY}/rpmbuild/

# Copy spec file
cp rpm/${BINARY}.spec target/rpm/${BINARY}/rpmbuild/SPECS/

cd target/rpm/${BINARY}/rpmbuild

rpmbuild -bb ./SPECS/${BINARY}.spec --build-in-place --define "_topdir $(pwd)" --define "version_ ${CRATE_VERSION}"

tree .
