#!/usr/bin/env python3

# SPDX-FileCopyrightText: © 2024 Dirk Sammel <dirk.sammel@gmail.com>
# SPDX-License-Identifier: BSD-2-Clause-Patent

import argparse

file_list = [
    "Cargo.toml",
    "auditor/Cargo.toml",
    "auditor-client/Cargo.toml",
    "pyauditor/Cargo.toml",
    "plugins/apel/pyproject.toml",
    "plugins/priority/Cargo.toml",
    "collectors/slurm/Cargo.toml",
    "collectors/slurm-epilog/Cargo.toml",
    "collectors/htcondor/pyproject.toml",
]


def bump_version(old_version: str, new_version: str) -> None:
    for file in file_list:
        with open(file, "r") as fin:
            s = fin.read()

        with open(file, "w") as fout:
            s = s.replace(old_version, new_version)
            fout.write(s)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-o", "--old", required=True, help="Old version")
    parser.add_argument("-n", "--new", required=True, help="New version")
    args = parser.parse_args()

    print(f"Replace version {args.old} with {args.new}.")
    answer = input("Are you sure? (y/n)\n")

    if answer == "y":
        bump_version(args.old, args.new)
        print(
            "Done! Other dependencies with the same version might have been updated unintentionally.\n"
            "Check with 'git diff' if everything is as expected."
        )
    else:
        print("Aborting.")


if __name__ == "__main__":
    main()
