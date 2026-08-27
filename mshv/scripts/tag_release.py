#!/usr/bin/env python3

import argparse
import re
import sys
from pathlib import Path
import subprocess

'''
Generates text for a tag like:
$ ./tag_release.py --crate mshv-bindings
mshv-bindings-v0.6.0

Added
* https://github.com/rust-vmm/mshv/pull/243 Add bindings for arm64 reset intercepts
* https://github.com/rust-vmm/mshv/pull/244 add function to query vmm capabilities

Fixed
* https://github.com/rust-vmm/mshv/pull/241 Fixes for arm64 guests
'''


def get_crate_path(crate, file):
    return Path(__file__).parent.parent / crate / file


def extract_changelog(crate, version):
    # Get the changelog file for the crate
    changelog_path = get_crate_path(crate, "CHANGELOG.md")
    with open(changelog_path, "r") as f:
        lines = f.readlines()

    # Find the changelog section for the specified version
    section_start = None
    section_end = None
    for i, line in enumerate(lines):
        if re.match(r"^## \[" + re.escape(version) + r"\]", line):
            section_start = i+1
        elif section_start is not None and re.match(r"^## \[", line):
            section_end = i
            break
    if section_start is None:
        raise RuntimeError("No changelog section found for version " + version)
    if section_end is None:
        section_end = len(lines)

    # Remove leading '#' from lines and strip whitespace
    section = [x.lstrip("#").rstrip().lstrip() for x in lines[section_start:section_end]]
    # Replace markdown links with just the url
    section = [re.sub(r"\[.*?\]\((.*?)\)", r"\1", x) for x in section]
    # Remove empty lines at start/end
    while section and not section[0].strip():
        section.pop(0)
    while section and not section[-1].strip():
        section.pop()
    return "\n".join(section)


def get_latest_version(crate):
    cargo_toml_path = get_crate_path(crate, "Cargo.toml")
    with open(cargo_toml_path, "r") as f:
        for line in f:
            m = re.match(r"^version = \"([^\"]+)\"", line)
            if m:
                return "v" + m.group(1)
    raise RuntimeError("No released version found in changelog")


def get_crates():
    root_cargo_toml = Path(__file__).parent.parent / "Cargo.toml"
    with open(root_cargo_toml, "r") as f:
        content = f.read()
    members_match = re.search(r'members\s*=\s*\[([^\]]+)\]', content)
    if not members_match:
        raise RuntimeError("No members found in root Cargo.toml")

    members_str = members_match.group(1)
    members = [m.strip().strip('"').strip("'") for m in members_str.split(",") if m.strip()]
    return members


def main(args):
    crates = []
    if not args.crate:
        crates = get_crates()
    else:
        crates = [args.crate]

    for crate in crates:
        print(f"Processing crate: {crate}")
        version = args.version if args.version else get_latest_version(crate)
        tag_name = f"{crate}-{version}"
        changelog = extract_changelog(crate, version)
        # create the tag with changelog as the message
        ret = subprocess.run(["git", "tag", "-a", tag_name, "-m", changelog])
        if ret.returncode != 0:
            raise RuntimeError(f"Error creating tag: {tag_name}")

        print(f"Created tag: {tag_name}")
    return 0


if __name__ == "__main__":
    parser = argparse.ArgumentParser(
        description="Generate the text for a new git tag based on the CHANGELOG.md",
        epilog="example:\n"
        "./tag_release.py --crate mshv-ioctls\n"
        "./tag_release.py --version v0.6.0 --crate mshv-bindings\n",
        formatter_class=argparse.RawTextHelpFormatter
    )
    parser.add_argument(
        "--version",
        "-v",
        type=str,
        dest="version",
        required=False,
        help="The version string, e.g. \"v0.6.0\", otherwise the current version in CARGO.toml",
    )
    parser.add_argument(
        "--crate",
        "-c",
        type=str,
        required=False,
        default=None,
        dest="crate",
        help="Generate tag for a crate (mshv-ioctls or mshv-bindings)",
    )
    res = main(parser.parse_args())
    sys.exit(res)
