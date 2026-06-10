#!/bin/bash

set -euo pipefail

cat <<EOF
Welcome to the rust-vmm interactive repository setup!

This script will guide you through initializing basic components of a fresh
rust-vmm repository, or alternatively lets you update the configuration of an
existing repository (for example, if since the first setup of a repository,
rust-vmm-ci has added new features that you would like to use).

EOF

RUST_VMM_CI=$(dirname ${BASH_SOURCE[0]})

SUPPORTED_PLATFORMS=("x86_64" "aarch64" "riscv64")
PLATFORMS_FILE=".platform"

DEPENDABOT_SCHEDULES=("weekly" "monthly")
DEPENDABOT_FILE=".github/dependabot.yml"

confirm() {
	read -p "$1 [Y/n] " -n 1 -r

	if [ ! -z $REPLY ]; then
		echo # move to new line unless user confirmed with just 'enter'
	else
		return 0
	fi
	[[ $REPLY =~ ^[Yy]$ ]]
}

setup_platforms_file() {
	touch $PLATFORMS_FILE
	echo "Please select the hardware plaforms for which you would like to enable CI support:"
	for platform in "${SUPPORTED_PLATFORMS[@]}"; do
		question="Enable support for $platform?"
		if confirm "$question"; then 
		       	echo $platform >> $PLATFORMS_FILE
		fi
	done
}

setup_dependabot_config() {
	mkdir -p $(dirname $DEPENDABOT_FILE)
	
	cat <<EOF
Dependabot allow you to automatically receive PRs for bumping your cargo
dependencies, as well as for updating the rust-vmm-ci submodule. You can choose
to run dependabot on different schedules: ${DEPENDABOT_SCHEDULES[@]}.

Which schedule would you like to enable? (say 'n' to disable dependabot)
EOF

	select opt in "${DEPENDABOT_SCHEDULES[@]}"
	do
		selected_config=$RUST_VMM_CI/dependabot-$opt.yml

		if [ -f $selected_config ]; then
			cp $selected_config $DEPENDABOT_FILE
			echo "Configured for $opt schedule"
		else
			echo "Not setting up dependabot"
		fi
		break
	done
}

if [ -f $PLATFORMS_FILE ]; then
	current_platforms=$(tr '\n' ' ' < $PLATFORMS_FILE)
	question="This repository already has a $PLATFORMS_FILE file setup. Do you want to regenerate it? Current supported platforms are: $current_platforms"

	if confirm "$question"; then
		rm $PLATFORMS_FILE
		setup_platforms_file
	fi
else
	setup_platforms_file
fi

echo

if [ -f "$DEPENDABOT_FILE" ]; then
	well_defined=0
	for schedule in "${DEPENDABOT_SCHEDULES[@]}"; do
		if cmp --silent $RUST_VMM_CI/dependabot-$schedule.yml $DEPENDABOT_FILE; then
			echo -n "Dependabot is already setup for the $schedule schedule. "
			well_defined=1
		fi
	done
	if [ $well_defined -eq 0 ]; then
		echo -n "Dependabot is already configured, although configuration does not match any rust-vmm-ci provided ones. "
	fi
	if confirm "Would you like to reconfigure dependabot?"; then
		rm $DEPENDABOT_FILE
		setup_dependabot_config
	fi
else
	setup_dependabot_config
fi

echo
echo "Setting up auto-publish for crates.io upon creation of GitHub tags..."

if [ ! -f Cargo.toml ]; then
	echo "Cargo.toml not found. Please re-run this script after setting up the crates to configure auto-publishing!"
else
	mkdir -p .github/workflows
	if grep -q '\[workspace\]' Cargo.toml; then
		members="$(cargo metadata --format-version 1 | jq '.workspace_members[]')"
		for member_path in $members; do
			member_path="${member_path#*//}"
			member_path="${member_path%#*}"

			crate_name=$(basename $member_path)
			# get path relative to crate root
			member_path=$(realpath -m --relative-to . $member_path)

			workflow_file=.github/workflows/publish-$crate_name.yml
			# Use printf when writing this value, so that whitespaces/newlines are respected
			workflow=$(sed "s/'v\*'/'$crate_name-v*'/" $RUST_VMM_CI/publish.yml | sed "s/cd ./cd $member_path/")

			if [ -f $workflow_file ] && cmp --silent $workflow_file <(printf "$workflow"); then
				echo "Publish workflow for $crate_name already setup, skipping"
				continue
			fi

			if confirm "Found workspace member '$crate_name' at '$member_path'. Setup auto-publish?"; then
				if [ -f $workflow_file ] && ! confirm "$workflow_file already exists. Overwrite?"; then
					continue
				fi

				printf "$workflow" > $workflow_file
				echo "If not already done, go to https://crates.io/crates/$crate_name/settings and add $(basename $workflow_file) as a trusted publisher!"
			fi
		done
	else
		question="Single crate repository detected, setup running 'cargo publish' at repository root when tags matching 'v*' are published? (if a .github/workflows/publish.yml file already exists, it will be overwritten)"
		if confirm "$question"; then
			cp $RUST_VMM_CI/publish.yml .github/workflows/publish.yml
		fi
	fi
fi
