#!/usr/bin/env bash
set -x
set -eo pipefail

BINARY=${BINARY:="auditor-slurm-epilog-collector"}
CRATE_VERSION=$(cargo get version)


mkdir -p target/rpm/${BINARY}/rpmbuild
mkdir -p target/rpm/${BINARY}/rpmbuild/BUILD
mkdir -p target/rpm/${BINARY}/rpmbuild/RPMS
mkdir -p target/rpm/${BINARY}/rpmbuild/SOURCES
mkdir -p target/rpm/${BINARY}/rpmbuild/SPECS
mkdir -p target/rpm/${BINARY}/rpmbuild/SRPMS

mkdir -p target/rpm/${BINARY}-$CRATE_VERSION

# cp target/x86_64-unknown-linux-musl/release/$BINARY target/rpm/$BINARY-$CRATE_VERSION/
# tar --create --file target/rpm/${BINARY}/rpmbuild/SOURCES/$BINARY-$CRATE_VERSION.tar.gz target/rpm/$BINARY-$CRATE_VERSION

cp target/x86_64-unknown-linux-musl/release/$BINARY target/rpm/$BINARY/rpmbuild/

cp rpm/$BINARY.spec target/rpm/$BINARY/rpmbuild/SPECS/

cd target/rpm/$BINARY/rpmbuild

# rpmbuild -ba --build-in-place --define "_topdir $(pwd)/rpm" helloworld.spec
rpmbuild -bb ./SPECS/$BINARY.spec --build-in-place --define "_topdir $(pwd)" --define "version_ ${CRATE_VERSION}"

tree .
