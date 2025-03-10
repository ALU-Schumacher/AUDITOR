#!/usr/bin/env python3
"""
Replace defined occurrences of version numbers with new numbers.
"""

import argparse
import functools
import re
from typing import Any

import tomlkit

# For .toml files we may specify the 'path' of a toml field.
# Then, we only try to update the specified field.

# A list of fields to update in the root Cargo.toml
cargo_toml_root_path = [
    ["workspace", "dependencies", "auditor", "version"],
    ["workspace", "dependencies", "auditor-client", "version"],
]
# The field to update in any other Cargo.toml
cargo_toml_path = ["package", "version"]
# The field to update in any pyproject.toml
pyproject_toml_path = ["project", "version"]

# A list of .toml files to update. Every filename is accompanied
# by a path-spec (or list of path-specs) that defines the
# element(s) to update.
# (mypy will complain if we don't provide the type here.)
tomls: list[tuple[str, list[Any]]] = [
    ("Cargo.toml", cargo_toml_root_path),
    #
    ("auditor/Cargo.toml", cargo_toml_path),
    ("auditor-client/Cargo.toml", cargo_toml_path),
    ("pyauditor/Cargo.toml", cargo_toml_path),
    ("plugins/priority/Cargo.toml", cargo_toml_path),
    ("collectors/kubernetes/Cargo.toml", cargo_toml_path),
    ("collectors/slurm/Cargo.toml", cargo_toml_path),
    ("collectors/slurm-epilog/Cargo.toml", cargo_toml_path),
    #
    ("plugins/apel/pyproject.toml", pyproject_toml_path),
    ("collectors/htcondor/pyproject.toml", pyproject_toml_path),
]

# A list of unstructured files to update.
# Every file is accompanied by a list of regex patterns.
# Each pattern is searched in the file and, if found, only the
# group defined in the pattern is interpreted as version string to update.
# Every pattern is expected to have exactly one group defined.
unstructured_files = [
    ("media/website/content/_index.md", [r"AUDITOR_VERSION\s*=\s*(\S*)"])
]


def nested_get(dic: dict, path: list) -> str:
    """
    Given a nested dict, obtain the element that is defined by the list `path`.
    I.e. if `path` is ["foo", "bar"], return dic["foo"]["bar"].

    Raises `KeyError` if the referenced element is not a string.
    """
    val = dic
    for key in path:
        val = val[key]
    if not isinstance(val, str):
        raise KeyError(f"Wrong dict path: {path}")
    return val


def nested_set(dic: dict, path: list, value: str):
    """
    Given a nested dict, set the element that is defined by the list `path`.
    Works analogous to `nested_get`.

    Raises `KeyError` if the referenced element is not a string.
    """
    d = dic
    for key in path[:-1]:
        d = d[key]
    if not isinstance(d[path[-1]], str):
        raise KeyError(f"Wrong dict path: {path}")
    d[path[-1]] = value


def bump_toml(fname: str, paths: list, old_version: str, new_version: str) -> None:
    """
    Update a single .toml file.
    Look for `old_version` and replace by `new_version` in every field
    defined by `paths`.
    """
    print(f"Edit {fname}")
    if not isinstance(paths[0], list):
        paths = [paths]

    with open(fname, "r", encoding="utf-8") as f:
        toml = tomlkit.parse(f.read())

    for path in paths:
        if nested_get(toml, path) == old_version:
            nested_set(toml, path, new_version)
        else:
            print(f"Strange version in {fname}: {nested_get(toml, path)}")

    with open(fname, "w", encoding="utf-8") as f:
        tomlkit.dump(toml, f)


def regex_replace(match: re.Match, old_version: str, new_version: str) -> str:
    """
    Helper replacement function to be passed (as partial) to re.sub.
    For a `match`, replace `old_version` by `new_version` in group(1).
    Return the updated full match.
    """
    if match.group(1) != old_version:
        print(f'Strange version {match.group(1)} in pattern "{match.re.pattern}"')
        return match.group(0)

    full_match = match.group(0)
    match_start = match.start()
    group_start, group_end = match.span(1)
    content = (
        full_match[: group_start - match_start]
        + new_version
        + full_match[group_end - match_start :]
    )
    return content


def bump_unstructured(
    fname: str, regexes: list[str], old_version: str, new_version: str
):
    """
    Update a single unstructured file.
    Look for every regex in `regexes`. For every match, consider only
    group(1) and - in this group - replace `old_version` by `new_version`.
    """
    print(f"Edit {fname}")
    with open(fname, "r", encoding="utf-8") as f:
        content = f.read()

    replace = functools.partial(
        regex_replace, old_version=old_version, new_version=new_version
    )
    for regex in regexes:
        _content = content
        content = re.sub(regex, replace, content)
        if _content == content:
            print(f'No match found for pattern "{regex}" in {fname}!')

    with open(fname, "w", encoding="utf-8") as f:
        f.write(content)


def bump_version(old_version: str, new_version: str):
    """Update all files defined above."""
    for file, path in tomls:
        bump_toml(file, path, old_version, new_version)
    for file, regexes in unstructured_files:
        bump_unstructured(file, regexes, old_version, new_version)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("-o", "--old", required=True, help="Old version")
    parser.add_argument("-n", "--new", required=True, help="New version")
    parser.add_argument(
        "-y", "--assumeyes", action="store_true", help="Don't ask for confirmation"
    )
    args = parser.parse_args()

    print(f"Replace version {args.old} with {args.new}.")
    answer = input("Are you sure? (y/N)\n") if not args.assumeyes else "y"

    if answer.lower() == "y":
        bump_version(args.old, args.new)
        print("Done! Please double check:")
        for file in unstructured_files:
            print(file[0])
    else:
        print("Aborting.")


if __name__ == "__main__":
    main()
