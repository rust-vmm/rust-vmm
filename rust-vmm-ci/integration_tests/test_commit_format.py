# Copyright 2020 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0
"""
Test the commit message format.

This test works properly on the local machine only when the environment
variables REMOTE and BASE_BRANCH are set. Otherwise the default values
are "origin" for the remote name of the upstream repository and "main"
for the name of the base branch, and this test may not work as expected.
"""

import os
import subprocess

from utils import get_cmd_output

COMMIT_TITLE_MAX_LEN = 60
COMMIT_BODY_LINE_MAX_LEN = 75
REMOTE = os.environ.get("BUILDKITE_REPO") or os.environ.get("REMOTE") or "origin"
BASE_BRANCH = (
    os.environ.get("BUILDKITE_PULL_REQUEST_BASE_BRANCH")
    or os.environ.get("BASE_BRANCH")
    or "main"
)


def test_commit_format():
    """
    Checks commit message format for the current PR's commits.

    Checks if commit messages do not have exceedingly long titles (a maximum 60
    characters for the title) and if commits are signed.
    """
    # Newer versions of git check the ownership of directories.
    # We need to add an exception for /workdir which is shared, so that
    # the git commands don't fail.
    config_cmd = "git config --global --add safe.directory /workdir"
    subprocess.run(config_cmd, shell=True, check=True)
    # Fetch the upstream repository.
    fetch_base_cmd = "git fetch {} {}".format(REMOTE, BASE_BRANCH)
    try:
        subprocess.run(fetch_base_cmd, shell=True, check=True)
    except subprocess.CalledProcessError:
        raise NameError(
            "The name of the base branch or remote is invalid. "
            "See test documentation for more details."
        ) from None
    # Get hashes of PR's commits in their abbreviated form for
    # a prettier printing.
    shas_cmd = "git log --no-merges --pretty=%h --no-decorate " "FETCH_HEAD..HEAD"
    shas = get_cmd_output(shas_cmd)

    for sha in shas.split():
        # Do not enforce the commit rules when the committer is dependabot.
        author_cmd = "git show -s --format='%ae' " + sha
        author = get_cmd_output(author_cmd)
        if "dependabot" in author:
            continue
        message_cmd = "git show --pretty=format:%B -s " + sha
        message = get_cmd_output(message_cmd)
        message_lines = message.split("\n")
        assert len(message_lines) >= 3, (
            "The commit '{}' should contain at least 3 lines: title, "
            "blank line and a sign-off one.".format(sha)
        )
        title = message_lines[0]
        assert message_lines[1] == "", (
            "For commit '{}', title is divided into multiple lines. "
            "Please keep it one line long and make sure you add a blank "
            "line between title and description.".format(sha)
        )
        assert len(title) <= COMMIT_TITLE_MAX_LEN, (
            "For commit '{}', title exceeds {} chars. "
            "Please keep it shorter.".format(sha, COMMIT_TITLE_MAX_LEN)
        )

        found_signed_off = False

        for line in message_lines[2:]:
            if line.startswith("Signed-off-by: "):
                found_signed_off = True
                break

        assert found_signed_off, (
            "Commit '{}' is not signed. "
            "Please run 'git commit -s --amend' "
            "on it.".format(sha)
        )
