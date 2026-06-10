# Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0
import pytest

PROFILE_CI = "ci"
PROFILE_DEVEL = "devel"

WORKSPACE = "workspace"
CRATE = "crate"


def pytest_addoption(parser):
    parser.addoption(
        "--profile",
        default=PROFILE_CI,
        choices=[PROFILE_CI, PROFILE_DEVEL],
        help="Profile for running the test: {} or {}".format(PROFILE_CI, PROFILE_DEVEL),
    )
    parser.addoption(
        "--no-cleanup",
        action="store_true",
        default=False,
        help="Keep the coverage report in `kcov_output` directory. If this "
        "flag is not provided, both coverage related directories are "
        "removed.",
    )

    parser.addoption(
        "--test-scope",
        default=WORKSPACE,
        choices=[WORKSPACE, CRATE],
        help="Defines the scope of running tests: {} or {}".format(WORKSPACE, CRATE),
    )


@pytest.fixture
def profile(request):
    return request.config.getoption("--profile")


@pytest.fixture
def no_cleanup(request):
    return request.config.getoption("--no-cleanup")


@pytest.fixture
def test_scope(request):
    return request.config.getoption("--test-scope")


# This is used for defining global variables in pytest.
def pytest_configure():
    # These constants are needed in tests, so this is the way that we can
    # export them.
    pytest.profile_ci = PROFILE_CI
    pytest.profile_devel = PROFILE_DEVEL
    pytest.workspace = WORKSPACE
    pytest.crate = CRATE
