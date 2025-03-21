#!/usr/bin/env python3
"""
Replace defined occurrences of version numbers with new numbers.
"""

import argparse
import functools
import json
import re
from typing import Any

import tomlkit

# For structured files (e.g. .toml)  we may specify the 'path' of a field.
# Then, we only try to update the specified field.
# Paths need to be accepted by `nested_get` below (read for details).

# A list of fields to update in the root Cargo.toml
cargo_toml_root_path = [
    ["workspace", "dependencies", "auditor", "version"],
    ["workspace", "dependencies", "auditor-client", "version"],
]
# The field to update in any other Cargo.toml
cargo_toml_path = ["package", "version"]
# The fields to update in any pyproject.toml
pyproject_toml_path = [
    ["project", "version"],
    ["project", "dependencies", re.compile(r"python-auditor\s*==\s*(\S+)")],
]
# The field to update in any rpm_config.json
rpm_config_path = ["core", "version"]


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

# A list of .json files to update. Works like the `tomls` list.
jsons: list[tuple[str, list[Any]]] = [
    ("plugins/apel/rpm_config.json", rpm_config_path),
    ("collectors/htcondor/rpm_config.json", rpm_config_path),
]

# A list of unstructured files to update.
# Every file is accompanied by a list of regex patterns.
# Each pattern is searched in the file and, if found, only the
# group defined in the pattern is interpreted as version string to update.
# Every pattern is expected to have exactly one group defined.
unstructured_files = [
    ("media/website/content/_index.md", [r"AUDITOR_VERSION\s*=\s*(\S+)"])
]


def _nested_get_ref(dic: dict, path: list):
    """
    Given a nested dict, obtain the element that is defined by the list `path`.
    I.e. if `path` is ["foo", "bar"], return dic["foo"]["bar"].

    Raises `KeyError` if any key is not a string.
    """
    d = dic
    for key in path:
        if not isinstance(key, str):
            raise KeyError("All keys except the last must be `str`")
        d = d[key]
    return d


def nested_get(dic: dict, path: list) -> str:
    """
    Given a nested dict, obtain the element that is defined by the list `path`.
    I.e. if `path` is ["foo", "bar"], return dic["foo"]["bar"].

    If the next to last element, i.e. `nested_get(dic, path[:-1])` is a list,
    the last key in `path` must be an `re.Pattern`. In this case,
    find the first matching element in the list
    and return group(1) of the match.

    Raises `KeyError` if the keys have wrong types or the
    referenced element is not a string.
    """
    ref = _nested_get_ref(dic, path[:-1])
    key = path[-1]
    if isinstance(key, str):
        if not isinstance(ref[key], str):
            raise KeyError(f"Wrong dict path: {path}")
        return ref[key]
    if isinstance(key, re.Pattern) and isinstance(ref, list):
        for elem in ref:
            m = re.fullmatch(key, elem)
            if m:
                return m.group(1)
        raise KeyError(f"Pattern {key} did not match")
    raise KeyError(f"Invalid key {key} for object {ref}")


def nested_set(dic: dict, path: list, value: str):
    """
    Given a nested dict, set the element that is defined by the list `path`.
    Works analogous to `nested_get`.
    """
    ref = _nested_get_ref(dic, path[:-1])
    key = path[-1]
    if isinstance(key, str):
        if not isinstance(ref[key], str):
            raise KeyError(f"Wrong dict path: {path}")
        ref[key] = value
    elif isinstance(key, re.Pattern) and isinstance(ref, list):
        for i, elem in enumerate(ref):
            m = re.fullmatch(key, elem)
            if m:
                ref[i] = elem[: m.start(1)] + value
                return
        raise KeyError(f"Pattern {key} did not match")
    else:
        raise KeyError(f"Invalid key {key} for object {ref}")


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

    changed = False
    for path in paths:
        if nested_get(toml, path) == old_version:
            nested_set(toml, path, new_version)
            changed = True
        else:
            print(f"Strange version in {fname}: {nested_get(toml, path)}")

    if changed:
        with open(fname, "w", encoding="utf-8") as f:
            tomlkit.dump(toml, f)


def bump_json(fname: str, paths: list, old_version: str, new_version: str) -> None:
    """
    Update a single .json file.
    Look for `old_version` and replace by `new_version` in every field
    defined by `paths`.
    """
    print(f"Edit {fname}")
    if not isinstance(paths[0], list):
        paths = [paths]

    with open(fname, "r", encoding="utf-8") as f:
        jsondata = json.load(f)

    changed = False
    for path in paths:
        if nested_get(jsondata, path) == old_version:
            nested_set(jsondata, path, new_version)
            changed = True
        else:
            print(f"Strange version in {fname}: {nested_get(jsondata, path)}")

    if changed:
        with open(fname, "w", encoding="utf-8") as f:
            json.dump(jsondata, f, indent=4)


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
    changed = False
    for regex in regexes:
        _content = content
        content = re.sub(regex, replace, content)
        if _content == content:
            print(f'No match found for pattern "{regex}" in {fname}!')
        else:
            changed = True

    if changed:
        with open(fname, "w", encoding="utf-8") as f:
            f.write(content)


def bump_version(old_version: str, new_version: str):
    """Update all files defined above."""
    for file, path in tomls:
        bump_toml(file, path, old_version, new_version)
    for file, path in jsons:
        bump_json(file, path, old_version, new_version)
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
