# Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0
"""Test the coverage and update the threshold when coverage is increased."""

import json
import os
import re
import shutil
import subprocess
import platform
import pytest

from utils import get_repo_root_path


def get_coverage_config_path():
    machine = platform.machine()
    target_file = f"coverage_config_{machine}.json"
    # We use a breadth-first search to guarantee that the config file
    # belongs to the crate that is being tested. Otherwise we might end
    # up wrongfully using the config file in the rust-vmm-ci submodule.
    # os.walkdir() offers a depth-first search and couldn't be used here.
    dirs = [os.getcwd()]
    while len(dirs):
        nextDirs = []
        for dir in dirs:
            for file in os.listdir(dir):
                file_path = os.path.join(dir, file)
                if os.path.isdir(file_path):
                    nextDirs.append(file_path)
                elif file == target_file:
                    return file_path
        dirs = nextDirs


REPO_ROOT_PATH = get_repo_root_path()
COVERAGE_CONFIG_PATH = get_coverage_config_path()


def _read_test_config():
    """
    Reads the config of the coverage for the repository being tested.

    Returns a JSON object with the configuration.
    """
    coverage_config = {}
    with open(COVERAGE_CONFIG_PATH) as config_file:
        coverage_config = json.load(config_file)

    assert "coverage_score" in coverage_config
    assert "exclude_path" in coverage_config

    if "crate_features" in coverage_config:
        assert (
            " " not in coverage_config["crate_features"]
        ), "spaces are not allowed in crate_features value"

    return coverage_config


def _write_coverage_config(coverage_config):
    """Updates the coverage config file as per `coverage_config`"""
    with open(COVERAGE_CONFIG_PATH, "w") as outfile:
        json.dump(coverage_config, outfile)


def _get_current_coverage(coverage_config, no_cleanup, test_scope):
    """Helper function that returns the coverage computed with llvm-cov."""
    # By default the build output for kcov and unit tests are both in the debug
    # directory. This causes some linker errors that I haven't investigated.
    # Error: error: linking with `cc` failed: exit code: 1
    # An easy fix is to have separate build directories for kcov & unit tests.
    cov_build_dir = os.path.join(REPO_ROOT_PATH, "cov_build")

    # Remove kcov output and build directory to be sure we are always working
    # on a clean environment.
    shutil.rmtree(cov_build_dir, ignore_errors=True)

    llvm_cov_command = (
        f"CARGO_TARGET_DIR={cov_build_dir} cargo llvm-cov test --json --summary-only"
    )

    additional_exclude_path = coverage_config["exclude_path"]
    if additional_exclude_path:
        llvm_cov_command += f' --ignore-filename-regex "{additional_exclude_path}"'

    if test_scope == pytest.workspace:
        llvm_cov_command += " --workspace "

    crate_features = coverage_config.get("crate_features")
    if crate_features:
        llvm_cov_command += " --features=" + crate_features
    if crate_features is None:
        llvm_cov_command += " --all-features"

    # Pytest closes stdin by default, but some tests might need it to be open.
    # In the future, should the need arise, we can feed custom data to stdin.
    result = subprocess.run(
        llvm_cov_command, shell=True, check=True, input=b"", stdout=subprocess.PIPE
    )

    summary = json.loads(result.stdout)
    coverage = summary["data"][0]["totals"]["lines"]["percent"]

    shutil.rmtree(cov_build_dir, ignore_errors=True)

    return coverage


def test_coverage(profile, no_cleanup, test_scope):
    MAX_DELTA = 0.5

    coverage_config = _read_test_config()
    current_coverage = _get_current_coverage(coverage_config, no_cleanup, test_scope)
    previous_coverage = coverage_config["coverage_score"]
    diff = current_coverage - previous_coverage
    upper = previous_coverage + MAX_DELTA
    arch = platform.machine()

    msg = (
        f"Current code coverage ({current_coverage:.2f}%) deviates by {diff:.2f}% from the previous code coverage {previous_coverage:.2f}%."
        f"Current code coverage must be within the range {previous_coverage:.2f}%..{upper:.2f}%."
        f"Please update the coverage in `coverage_config_{arch}.json`."
    )

    if abs(diff) > MAX_DELTA:
        if previous_coverage < current_coverage:
            if profile == pytest.profile_ci:
                # In the CI Profile we expect the coverage to be manually updated.
                raise ValueError(msg)
            elif profile == pytest.profile_devel:
                coverage_config["coverage_score"] = current_coverage
                _write_coverage_config(coverage_config)
            else:
                # This should never happen because pytest should only accept
                # the valid test profiles specified with `choices` in
                # `pytest_addoption`.
                raise RuntimeError("Invalid test profile.")
        elif previous_coverage > current_coverage:
            raise ValueError(msg)
