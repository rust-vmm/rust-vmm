#!/usr/bin/env python3

# Copyright 2021 Amazon.com, Inc. or its affiliates. All Rights Reserved.
# SPDX-License-Identifier: Apache-2.0 OR BSD-3-Clause
"""
This script is printing the Buildkite pipeline.yml to stdout.
This can also be used as a library to print the steps from a different pipeline
specified as a parameter to the `generate_test_pipeline`.

The pipeline is generated based on the test configuration in
`test_description.json`. The JSON file contains a list of tests to be run by
all rust-vmm components. Each test has a default timeout of 5 minutes.

Some components need to override the default configuration such that they can
access devices while running the tests (for example access to `/dev/kvm`),
access to a temporary volume, and others. Some components may also need to skip
some of the tests. As such, this script supports overriding the following
configurations through environment variables:
- `X86_LINUX_AGENT_TAGS`: overrides the tags by which the x86_64 linux agent is
  selected.
- `AARCH64_LINUX_AGENT_TAGS`: overrides the tags by which the aarch64 linux
  agent is selected.
- `DOCKER_PLUGIN_CONFIG`: specifies additional configuration for the docker
  plugin. For available configuration, please check the
  https://github.com/buildkite-plugins/docker-buildkite-plugin.
- `TESTS_TO_SKIP`: specifies a list of tests to be skipped.
- `TIMEOUTS_MIN`: overrides the timeout value for specific tests.

NOTE: The variable `TESTS_TO_SKIP` is specified as a JSON list with the names
of the tests to be skipped. The variable `TIMEOUTS_MIN` is a dictionary where
each key is the name of a test and each value is the number of minutes for the
timeout. The other variables are specified as dictionaries, where the first key
is `tests` and its value is a list of test names where the configuration should
be applied; the second key is `cfg` and its value is a dictionary with the
actual configuration.

Examples of a valid configuration:
```shell
TESTS_TO_SKIP='["commit-format"]'
DOCKER_PLUGIN_CONFIG='{
    "tests": ["coverage"],
    "cfg": {
        "devices": [ "/dev/vhost-vdpa-0" ],
        "privileged": true
    }
}'
TIMEOUTS_MIN='["style": 30]'
```

When invoked with `--workspace-selective`, the script emits one step per
workspace crate tagged with Buildkite's native `if_changed` property, so each
crate's shared crate-scoped tests only run when files in that crate (or in a
workspace-global path) change. Tests tagged with `"scope": "workspace"`
(e.g. `commit-format`) have no `if_changed` and always run.
"""

import yaml
import json
import os
import sys
import pathlib
import copy
import glob
import tomllib

from argparse import ArgumentParser, RawTextHelpFormatter
from textwrap import dedent

# This represents the version of the rust-vmm-container used
# for running the tests.
CONTAINER_VERSION = "g2b5b066"
# The suffix suggests that the dev image with `v{N}-riscv` tag is not to be
# confused with real `riscv64` image (it's actually a `x86_64` image with
# `qemu-system-riscv64` installed), since AWS yet has `riscv64` machines
# available.
CONTAINER_VERSION_RISCV = CONTAINER_VERSION + "-riscv"
# This represents the version of the Buildkite Docker plugin.
DOCKER_PLUGIN_VERSION = "v5.3.0"

X86_AGENT_TAGS = os.getenv("X86_LINUX_AGENT_TAGS")
AARCH64_AGENT_TAGS = os.getenv("AARCH64_LINUX_AGENT_TAGS")
DOCKER_PLUGIN_CONFIG = os.getenv("DOCKER_PLUGIN_CONFIG")
TESTS_TO_SKIP = os.getenv("TESTS_TO_SKIP")
TIMEOUTS_MIN = os.getenv("TIMEOUTS_MIN")
# This env allows setting the hypervisor on which the tests are running at the
# pipeline level. This will not override the hypervisor tag in case one is
# already specified in the test definition.
# Most of the repositories don't really need to run on KVM per se, but we are
# experiencing some timeouts mostly with the mshv hosts right now, and we are
# fixing the default to kvm to work around that problem.
# More details here: https://github.com/rust-vmm/community/issues/137
DEFAULT_AGENT_TAG_HYPERVISOR = os.getenv("DEFAULT_AGENT_TAG_HYPERVISOR", "kvm")

BUILDKITE_PATH = pathlib.Path(__file__).parent.resolve()

# Per-crate file listing the platforms CI should run on.
PLATFORMS_FILE = ".platform"

# Paths added to every shared crate-scoped step's `if_changed`, so a change to
# the workspace as a whole fires each crate's shared tests.
WORKSPACE_GLOBAL_PATHS = ("Cargo.toml", ".buildkite/**", "rust-vmm-ci/**")


class BuildkiteStep:
    """
    This builds a Buildkite step according to a json configuration and the
    environment variables `X86_LINUX_AGENT_TAGS`, `AARCH64_LINUX_AGENT_TAGS`,
    `DOCKER_PLUGIN_CONFIG`, `TESTS_TO_SKIP` and `TIMEOUTS_MIN`.
    The output is a dictionary.
    """

    def __init__(self):
        """
        Initialize a Buildkite step with default values.
        """
        # Default values.
        # The order in which the attributes are initialized is the same as the
        # order in which the keys will appear in the YAML file, because Python
        # dictionaries are ordered. For readability reasons, this order should
        # not be changed.
        self.step_config = {
            "label": None,
            "command": None,
            "retry": {"automatic": False},
            "agents": {"os": "linux"},
            "plugins": [
                {
                    f"docker#{DOCKER_PLUGIN_VERSION}": {
                        "image": f"rustvmm/dev:{CONTAINER_VERSION}",
                        "always-pull": True,
                    }
                }
            ],
            "timeout_in_minutes": 15,
        }

    def _set_platform(self, platform):
        """Set platform if given in the json input."""

        if platform:
            # We need to change `aarch64` to `arm` because of the way we are
            # setting the tags on the host.
            if platform == "aarch64":
                platform = "arm"
            self.step_config["agents"]["platform"] = f"{platform}.metal"

    def _set_hypervisor(self, hypervisor):
        """Set hypervisor if given in the json input."""
        supported_hypervisors = ["kvm", "mshv"]
        if hypervisor:
            if hypervisor in supported_hypervisors:
                self.step_config["agents"]["hypervisor"] = hypervisor

    def _set_conditional(self, conditional):
        """Set conditional if given in the json input."""

        if conditional:
            self.step_config["if"] = conditional

    def _set_timeout_in_minutes(self, timeout):
        """Set the timeout if given in the json input."""
        if timeout:
            self.step_config["timeout_in_minutes"] = timeout

    def _set_agent_queue(self, queue):
        """Set the agent queue if provided in the json input."""
        if queue:
            self.step_config["agents"]["queue"] = queue

    def _add_docker_config(self, cfg):
        """Add configuration for docker if given in the json input."""

        if cfg:
            target = self.step_config["plugins"][0][f"docker#{DOCKER_PLUGIN_VERSION}"]
            for key, val in cfg.items():
                target[key] = val

    def _env_change_config(self, test_name, env_var, target, override=False):
        """
        Helper function to add to/override configuration of `target`
        if `env_var` is set and this test appears in its list.
        """

        if env_var:
            env_cfg = json.loads(env_var)

            tests = env_cfg.get("tests")
            assert tests, f"Environment variable {env_var} is missing the `tests` key."

            cfg = env_cfg.get("cfg")
            assert cfg, f"Environment variable {env_var} is missing the `cfg` key."

            if test_name in tests:
                if override:
                    target.clear()
                for key, val in cfg.items():
                    target[key] = val

    def _env_override_agent_tags(self, test_name):
        """
        Override the tags by which the linux agent is selected
        using the `X86_LINUX_AGENT_TAGS` and `AARCH64_LINUX_AGENT_TAGS`
        environment variables.
        """

        env_var = None
        platform = self.step_config["agents"].get("platform")

        # Since the platform is optional, only override the config if the
        # platform was provided.
        if platform:
            if platform == "x86_64.metal" and X86_AGENT_TAGS:
                env_var = X86_AGENT_TAGS
            if platform == "arm.metal" and AARCH64_AGENT_TAGS:
                env_var = AARCH64_AGENT_TAGS

        target = self.step_config["agents"]
        self._env_change_config(test_name, env_var, target, override=True)

    def _env_add_docker_config(self, test_name):
        """
        Specify additional configuration for the docker plugin using the
        `DOCKER_PLUGIN_CONFIG` environment variable.
        """

        target = self.step_config["plugins"][0][f"docker#{DOCKER_PLUGIN_VERSION}"]
        self._env_change_config(test_name, DOCKER_PLUGIN_CONFIG, target)

    def _env_override_timeout(self, test_name):
        if TIMEOUTS_MIN:
            timeouts_min = json.loads(TIMEOUTS_MIN)
            if test_name in timeouts_min:
                self.timeout_in_minutes = timeouts_min[test_name]

    def build(self, input):
        """
        Build a Buildkite step using the `input` configuration that must
        specify some mandatory keys and can also provide optional ones.
        Further configuration from environment variables may be added.
        """

        test_name = input.get("test_name")
        command = input.get("command")
        platform = input.get("platform")
        hypervisor = input.get("hypervisor")
        docker = input.get("docker_plugin")
        conditional = input.get("conditional")
        timeout = input.get("timeout_in_minutes")
        queue = input.get("queue")
        crate = input.get("crate")
        crate_path = input.get("crate_path")

        # Mandatory keys.
        assert test_name, "Step is missing test name."
        platform_string = f"-{platform}" if platform else ""
        # When the step is crate-scoped, prefix the label with the crate to disambiguate.
        crate_string = f"{crate}: " if crate else ""
        self.step_config["label"] = f"{crate_string}{test_name}{platform_string}"

        assert command, "Step is missing command."
        if "{target_platform}" in command:
            assert platform, "Command requires platform, but platform is missing."
            command = command.replace("{target_platform}", platform)
        if "{crate}" in command:
            assert crate, "Command requires crate, but crate is missing."
            command = command.replace("{crate}", crate)
        if "{crate_path}" in command:
            assert crate_path, "Command requires crate path, but crate path is missing."
            command = command.replace("{crate_path}", crate_path)
        # Modify command and tag name for `riscv64` CI
        if platform == "riscv64":
            # Wrap command with '' to avoid escaping early by `ENTRYPOINT`
            command = json.dumps(command)
            # Overwrite image tag for riscv64 platform CI
            self.step_config["plugins"][0][f"docker#{DOCKER_PLUGIN_VERSION}"][
                "image"
            ] = f"rustvmm/dev:{CONTAINER_VERSION_RISCV}"
            # Since we are using qemu-system inside a x86_64 container, we
            # should set `platform` field to x86_64 and unset the hypervisor to
            # be passed
            platform = "x86_64"
            hypervisor = ""
        self.step_config["command"] = command

        # Optional keys.
        self._set_platform(platform)
        self._set_hypervisor(hypervisor)
        self._set_conditional(conditional)
        self._add_docker_config(docker)
        self._set_timeout_in_minutes(timeout)
        self._set_agent_queue(queue)

        # Override/add configuration from environment variables.
        self._env_override_agent_tags(test_name)
        self._env_add_docker_config(test_name)
        self._env_override_timeout(test_name)

        # We're now adding the keys for which we don't have explicit support
        # (i.e. there is no checking/updating taking place). We are just
        # forwarding the key, values without any change.
        # We need to filter for keys that have special meaning and which we
        # don't want to re-add.
        special_keys = [
            "conditional",
            "docker_plugin",
            "platform",
            "test_name",
            "queue",
            "hypervisor",
            "crate",
            "crate_path",
            "scope",
        ]
        additional_keys = {
            k: v
            for k, v in input.items()
            if not (k in self.step_config) and not (k in special_keys)
        }
        if additional_keys:
            self.step_config.update(additional_keys)

        # Return the object's attributes and their values as a dictionary.
        return self.step_config


class BuildkiteConfig:
    """
    This builds the final Buildkite configuration from the json input
    using BuidkiteStep objects. The output is a dictionary that can
    be put into yaml format by the pyyaml package.
    """

    def __init__(self):
        self.bk_config = None

    @staticmethod
    def _skip_test(test_name):
        """Return whether `test_name` is excluded via the `TESTS_TO_SKIP` env."""
        return bool(TESTS_TO_SKIP) and test_name in json.loads(TESTS_TO_SKIP)

    def _append_test_steps(
        self, test, platform_allowlist, crate=None, crate_path=None, if_changed=None
    ):
        """Append the steps for `test`, one per allowed platform, scoped to `crate`."""
        platforms = test.get("platform")
        # The platform is optional.
        if not platforms:
            platforms = [None]

        for platform in platforms:
            # Filter test enabled in platform_allowlist
            if platform is not None and platform not in platform_allowlist:
                # Skip disabled platform
                continue

            step_input = copy.deepcopy(test)
            step_input["platform"] = platform
            step_input["crate"] = crate
            step_input["crate_path"] = crate_path
            if if_changed is not None:
                step_input["if_changed"] = if_changed
            if not step_input.get("hypervisor"):
                step_input["hypervisor"] = DEFAULT_AGENT_TAG_HYPERVISOR

            step = BuildkiteStep()
            self.bk_config["steps"].append(step.build(step_input))

    def build(self, input, platform_allowlist):
        """Build the final Buildkite configuration fron the json input."""

        self.bk_config = {"steps": []}
        tests = input.get("tests")
        assert tests, "Input is missing list of tests."

        for test in tests:
            if self._skip_test(test.get("test_name")):
                continue
            self._append_test_steps(test, platform_allowlist)

        # Return the object's attributes and their values as a dictionary.
        return self.bk_config

    def _append_crate_steps(self, test, crate, crate_path, if_changed=None):
        """Append `test` scoped to `crate`, using the crate's own `.platform`."""
        allowlist = determine_allowlist(os.path.join(crate_path, PLATFORMS_FILE))
        self._append_test_steps(
            test,
            allowlist,
            crate=crate,
            crate_path=crate_path,
            if_changed=if_changed,
        )

    def build_workspace_selective(self, input, dirs, root_allowlist):
        """Build the workspace-selective configuration with `if_changed` per step."""

        self.bk_config = {"steps": []}
        tests = input.get("tests")
        assert tests, "Input is missing list of tests."

        # Shared tests: workspace-scoped ones always run; crate-scoped ones run
        # per crate and fire on changes to that crate or a workspace-global path.
        for test in tests:
            if self._skip_test(test.get("test_name")):
                continue
            if test.get("scope", "crate") == "workspace":
                self._append_test_steps(test, root_allowlist)
                continue
            for crate in sorted(dirs):
                crate_path = dirs[crate]
                self._append_crate_steps(
                    test,
                    crate,
                    crate_path,
                    if_changed=[f"{crate_path}/**", *WORKSPACE_GLOBAL_PATHS],
                )

        # Per-crate extras fire only on changes inside that crate (decoupled
        # from workspace-global paths so a root change does not retrigger them).
        for crate in sorted(dirs):
            crate_path = dirs[crate]
            for test in crate_test_description(crate_path):
                if self._skip_test(test.get("test_name")):
                    continue
                self._append_crate_steps(
                    test, crate, crate_path, if_changed=[f"{crate_path}/**"]
                )

        # Return the object's attributes and their values as a dictionary.
        return self.bk_config


def determine_allowlist(config_file):
    """Determine the what platforms should be enabled for this crate"""

    try:
        with open(config_file, "r") as file:
            platforms = [line.strip() for line in file.readlines()]
        return platforms
    except Exception as e:
        # Fall back to default platform if anything goes wrong
        return ["x86_64", "aarch64"]


def crate_test_description(crate_path):
    """Return the crate's own extra tests from `<crate>/.buildkite/`, if any."""
    path = os.path.join(crate_path, ".buildkite", "test_description.json")
    if not os.path.exists(path):
        return []
    with open(path) as json_file:
        return json.load(json_file).get("tests", [])


def workspace_members():
    """Return `{crate_name: relative_path}` for each workspace member.

    Parses the workspace's Cargo.toml manifests with the stdlib `tomllib`
    module so this script does not require `cargo` on the host.
    """
    with open("Cargo.toml", "rb") as f:
        patterns = tomllib.load(f).get("workspace", {}).get("members", [])

    members = {}
    for pattern in patterns:
        # Expand glob patterns like `crates/*`; bare paths pass through.
        matches = (
            sorted(glob.glob(pattern))
            if any(c in pattern for c in "*?[")
            else [pattern]
        )
        for member_dir in matches:
            manifest = os.path.join(member_dir, "Cargo.toml")
            if not os.path.isfile(manifest):
                continue
            with open(manifest, "rb") as f:
                name = tomllib.load(f).get("package", {}).get("name")
            if name:
                members[name] = member_dir
    return members


def generate_pipeline(config_file, platform_allowlist):
    """Generate the pipeline yaml file from a json configuration file."""

    with open(config_file) as json_file:
        json_cfg = json.load(json_file)
        json_file.close()

    config = BuildkiteConfig()
    output = config.build(json_cfg, platform_allowlist)
    yaml.dump(output, sys.stdout, sort_keys=False)


def generate_workspace_selective_pipeline(config_file, root_allowlist):
    """Generate a workspace-selective pipeline from a json configuration file."""

    with open(config_file) as json_file:
        json_cfg = json.load(json_file)

    dirs = workspace_members()

    config = BuildkiteConfig()
    output = config.build_workspace_selective(json_cfg, dirs, root_allowlist)
    yaml.dump(output, sys.stdout, sort_keys=False)


if __name__ == "__main__":
    help_text = dedent("""
        This script supports overriding the following configurations through
        environment variables:
        - X86_LINUX_AGENT_TAGS: overrides the tags by which the x86_64 linux
        agent is selected.
        - AARCH64_LINUX_AGENT_TAGS: overrides the tags by which the aarch64
        linux agent is selected.
        - DOCKER_PLUGIN_CONFIG: specifies additional configuration for the
        docker plugin. For available configuration, please check
        https://github.com/buildkite-plugins/docker-buildkite-plugin.
        - TESTS_TO_SKIP: specifies a list of tests to be skipped.
        - TIMEOUTS_MIN: overrides the timeout value for specific tests.
        """)
    parser = ArgumentParser(description=help_text, formatter_class=RawTextHelpFormatter)
    # By default we're generating the rust-vmm-ci pipeline with the test
    # configuration committed to this repository.
    # This parameter is useful for generating the pipeline for repositories
    # that have custom pipelines, and it helps with keeping the container
    # version the same across pipelines.
    parser.add_argument(
        "-t",
        "--test-description",
        metavar="JSON_FILE",
        help="The path to the JSON file containing the test" " description for the CI.",
        default=f"{BUILDKITE_PATH}/test_description.json",
    )
    parser.add_argument(
        "-p",
        "--platform-allowlist",
        metavar="PLATFORM_DOT_FILE",
        help=(
            "The path to the dotfile containing platforms the crate's CI should run on.\n"
            "If the file does not exist, falls back to default `platform_allowlist` (x86_64 and arm64).\n"
            "The dotfile contains strings of architectures to be enabled separated by\n"
            "newlines."
        ),
        default=f"{os.getcwd()}/.platform",
    )
    # `--workspace-selective` takes a value rather than being a flag (no
    # `action="store_true"`) because the infrastructure that invokes this
    # script supports parameters with arguments only, not bare options.
    parser.add_argument(
        "--workspace-selective",
        default="False",
        metavar="BOOL",
        help=(
            "When 'True', generate a selective pipeline for a Cargo workspace,\n"
            "emitting one step per crate gated by Buildkite's `if_changed`\n"
            "property so each crate's shared tests only run when files in that\n"
            "crate (or a workspace-global path) change. The per-crate test\n"
            "description (using the {crate}/{crate_path} placeholders and\n"
            "`scope`) is supplied via -t. Default: 'False'."
        ),
    )
    args = parser.parse_args()
    platform_allowlist = determine_allowlist(args.platform_allowlist)
    if args.workspace_selective == "True":
        generate_workspace_selective_pipeline(args.test_description, platform_allowlist)
    else:
        generate_pipeline(args.test_description, platform_allowlist)
