#!/usr/bin/env python3
"""
Check for inactive codeowners in GitHub repositories.

This script fetches the CODEOWNERS file from one or more GitHub repositories
and checks if any codeowners have had no activity (reviews, PR comments,
issue comments) over a specified time period (default: past year).

Dependencies:
    pip install -r scripts/requirements.txt

Usage:
    # Check a single repository
    python check_inactive_codeowners.py --org rust-vmm --repos vhost

    # Check multiple repositories
    python check_inactive_codeowners.py --org rust-vmm --repos vhost,virtio-vsock,vm-memory

    # Check all repositories in an organization
    python check_inactive_codeowners.py --org rust-vmm

    # Check with custom time period and verbose output
    python check_inactive_codeowners.py --org myorg --repos myrepo --days 180 --verbose

    # Check activity in a specific date range
    python check_inactive_codeowners.py --org rust-vmm --days 365 --until 2024-12-31

Returns:
    0 if all codeowners are active
    1 if there are inactive codeowners
    2 if there are errors while querying GitHub
"""

import argparse
import logging
import os
import re
import sys
from datetime import datetime, timedelta, timezone

try:
    from github import Github, Auth, GithubException
except ImportError:
    print(
        "Error: PyGithub is not installed. Install it with: pip install PyGithub",
        file=sys.stderr,
    )
    sys.exit(2)

logger = logging.getLogger(__name__)


def parse_codeowners(codeowners_content):
    """Parse CODEOWNERS content and extract GitHub usernames.

    Parses the CODEOWNERS file format and extracts individual GitHub user references.
    Skips comments, empty lines, and team references (@org/team-name).

    Args:
        codeowners_content: String content of a CODEOWNERS file

    Returns:
        set: Set of GitHub usernames (without @ prefix)

    Note:
        Only individual users are extracted. Team references are currently not supported.
    """
    usernames = set()

    for line in codeowners_content.split("\n"):
        line = line.strip()

        if not line or line.startswith("#"):
            continue

        # Extract @username patterns, skip team references (@org/team-name)
        # GitHub usernames can contain alphanumeric and dashes
        # Only match if followed by whitespace or end of string (not /)
        matches = re.findall(r"@([a-zA-Z0-9-]+)(?=\s|$)", line)
        usernames.update(matches)

    return usernames


def check_user_exists(gh, username):
    """Check if a GitHub user exists.

    Args:
        gh: PyGithub Github instance
        username: GitHub username to check

    Returns:
        bool: True if user exists, False otherwise

    Note:
        Uses the GitHub Users API to verify user existence.
    """
    try:
        gh.get_user(username)
        logger.debug("User %s exists", username)
        return True
    except GithubException as e:
        if e.status == 404:
            logger.debug("User %s not found (404)", username)
            return False

        logger.warning("Unexpected error checking user %s: %s", username, e)
        return True  # Assume exists to avoid false positives
    except Exception as e:
        logger.warning("Error checking if user exists: %s", e)
        return True  # Assume exists on error to avoid false positives


def fetch_codeowners_from_github(gh, org, repo, codeowners_path=None):
    """Fetch CODEOWNERS file from GitHub repository.

    Tries common CODEOWNERS locations if a specific path is not provided.

    Args:
        gh: PyGithub Github instance
        org: GitHub organization name
        repo: Repository name
        codeowners_path: Specific path to CODEOWNERS file (optional)

    Returns:
        str or None: Content of the CODEOWNERS file, or None if not found

    Note:
        If codeowners_path is not specified, tries these locations in order:
        - CODEOWNERS (root)
        - .github/CODEOWNERS
        - docs/CODEOWNERS
    """
    locations = (
        [codeowners_path]
        if codeowners_path
        else ["CODEOWNERS", ".github/CODEOWNERS", "docs/CODEOWNERS"]
    )

    repository = gh.get_repo(f"{org}/{repo}")

    for location in locations:
        try:
            content = repository.get_contents(location)
            if content and hasattr(content, "decoded_content"):
                return content.decoded_content.decode("utf-8")

            logger.debug("CODEOWNERS not found at %s", location)
        except Exception as e:
            logger.debug("Failed to fetch CODEOWNERS from %s: %s", location, e)
            continue

    return None


def process_user_activity(
    items,
    item_label,
    usernames,
    author,
    since_date,
    until_date,
    user_activity,
    activity_key,
    item_number,
):
    """Process reviews or comments and track unique user activity.

    Args:
        items: List of review or comment objects
        item_label: 'review' or 'comment' for date handling and logging
        usernames: Set of usernames to track
        author: Author of PR/issue (skip self-activity)
        since_date: Start of date range
        until_date: End of date range
        user_activity: Dictionary to update
        activity_key: Key to increment in user_activity
        item_number: PR/issue number for debug logging
    """
    users_counted = set()

    for item in items:
        if not item.user:
            continue
        user_login = item.user.login

        # Skip if: not a tracked codeowner, activity on own PR/issue, or already counted
        if (
            user_login not in usernames
            or user_login == author
            or user_login in users_counted
        ):
            continue

        if item_label == "review":
            item_date = (
                item.submitted_at.replace(tzinfo=timezone.utc)
                if item.submitted_at
                else None
            )
            if not item_date:
                continue
        else:
            item_date = item.created_at.replace(tzinfo=timezone.utc)

        if item_date < since_date or item_date > until_date:
            continue

        users_counted.add(user_login)
        user_activity[user_login][activity_key] += 1
        logger.debug("Found %s by %s on #%s", item_label, user_login, item_number)


def process_items_for_activity(
    gh, query, usernames, since_date, until_date, user_activity, item_type
):
    """Process PRs or issues to track user activity.

    Counts the number of unique PRs/issues where each user participated
    (via reviews or comments). Each user is counted at most once per
    PR/issue.

    Args:
        gh: PyGithub Github instance
        query: GitHub search query string
        usernames: Set of usernames to check for activity
        since_date: Timezone-aware datetime to check activity from
        until_date: Timezone-aware datetime to check activity until
        user_activity: Dictionary to update with activity counts
        item_type: Either 'pr' or 'issue' for logging and progress messages
    """
    activity_key = f"{item_type}_commented"
    item_label = item_type.upper()

    try:
        items = gh.search_issues(query)
        total_count = items.totalCount

        for idx, item in enumerate(items, 1):
            print(
                f"  Searching for {item_label} activity... {idx}/{total_count}",
                end="\r",
                flush=True,
            )

            try:
                author = item.user.login if item.user else None

                # Handle PR reviews
                if item_type == "pr":
                    reviews = list(item.as_pull_request().get_reviews())
                    process_user_activity(
                        reviews,
                        "review",
                        usernames,
                        author,
                        since_date,
                        until_date,
                        user_activity,
                        "pr_reviewed",
                        item.number,
                    )

                # Handle PR/Issue comments
                comments = list(item.get_comments())
                process_user_activity(
                    comments,
                    "comment",
                    usernames,
                    author,
                    since_date,
                    until_date,
                    user_activity,
                    activity_key,
                    item.number,
                )
            except Exception as e:
                logger.warning(
                    "Could not fetch data for %s #%s: %s", item_label, item.number, e
                )

        print()  # Clear the progress line
    except Exception as e:
        logger.warning("Batch %s activity search failed: %s", item_label, e)


def check_repository(gh, org, repo, since_date, until_date, include_commits=False):
    """Check a single repository for inactive codeowners.

    Uses batched GitHub API queries to efficiently check all codeowners
    for activity (PR reviews, comments, and optionally commits).

    Args:
        gh: PyGithub Github instance
        org: GitHub organization name
        repo: Repository name
        since_date: Timezone-aware datetime to check activity from
        until_date: Timezone-aware datetime to check activity until
        include_commits: If True, include commits in activity check

    Returns:
        dict or None: Dictionary with keys 'inactive', 'active', 'nonexistent':
            - 'inactive': list of usernames with no activity
            - 'active': list of tuples (username, activity_dict) where activity_dict contains:
                - 'pr_commented': Number of PRs where user commented
                - 'issue_commented': Number of issues where user commented
                - 'pr_reviewed': Number of PRs where user left a review
                - 'commits': Number of commits (only if include_commits=True)
                - 'active': Boolean indicating if user has any activity
            - 'nonexistent': list of usernames that don't exist on GitHub
        Returns None if no CODEOWNERS file found or no codeowners defined.
    """
    repository = gh.get_repo(f"{org}/{repo}")

    if repository.archived:
        print("  Repository is archived, skipping")
        return None
    codeowners_content = fetch_codeowners_from_github(gh, org, repo)
    if not codeowners_content:
        print("  No CODEOWNERS file found, skipping")
        return None

    usernames = parse_codeowners(codeowners_content)
    if not usernames:
        print("  CODEOWNERS file is empty, skipping")
        return None

    print(f"Found {len(usernames)} codeowners, checking if users exist...")
    valid_usernames = set()
    nonexistent_users = []

    for username in usernames:
        if not check_user_exists(gh, username):
            nonexistent_users.append(username)
        else:
            valid_usernames.add(username)

    nonexistent_users.sort()

    if nonexistent_users:
        print(f"  Non-existent users: {', '.join(nonexistent_users)}")
    if valid_usernames:
        print(f"  Valid users: {', '.join(sorted(valid_usernames))}")

    if not valid_usernames:
        return {"inactive": [], "active": [], "nonexistent": nonexistent_users}

    user_activity = {
        username: {
            "pr_commented": 0,
            "issue_commented": 0,
            "pr_reviewed": 0,
        }
        for username in valid_usernames
    }

    if include_commits:
        print("Checking commits...")
        for username in valid_usernames:
            logger.debug("Checking commits for %s", username)
            commits = repository.get_commits(
                author=username, since=since_date, until=until_date
            )
            count = commits.totalCount
            user_activity[username]["commits"] = count
            logger.debug("Commits for %s: %s", username, count)

    reviewed_str = " ".join([f"reviewed-by:{u}" for u in valid_usernames])
    commenter_str = " ".join([f"commenter:{u}" for u in valid_usernames])
    date_range = f"{since_date.date().isoformat()}..{until_date.date().isoformat()}"

    # Batch search for all PR activity (reviews and comments)
    print("Checking PR activity...")
    pr_query = (
        f"repo:{org}/{repo} is:pr {reviewed_str} {commenter_str} updated:{date_range}"
    )
    logger.debug("PR query: %s", pr_query)
    process_items_for_activity(
        gh, pr_query, valid_usernames, since_date, until_date, user_activity, "pr"
    )

    # Batch search for issue comments
    print("Checking issue activity...")
    issue_query = f"repo:{org}/{repo} is:issue {commenter_str} updated:{date_range}"
    logger.debug("Issue query: %s", issue_query)
    process_items_for_activity(
        gh, issue_query, valid_usernames, since_date, until_date, user_activity, "issue"
    )

    inactive_users = []
    active_users = []

    for username in sorted(valid_usernames):
        activity = user_activity[username]
        activity["active"] = (
            activity.get("commits", 0) > 0
            or activity["pr_commented"] > 0
            or activity["issue_commented"] > 0
            or activity["pr_reviewed"] > 0
        )

        if not activity["active"]:
            inactive_users.append(username)
        else:
            active_users.append((username, activity))

    print("Results:")
    if active_users:
        print("  Active users:")
        for username, activity in active_users:
            print(f"    {username}: {activity}")

    if inactive_users:
        print("  Inactive users:")
        for username in inactive_users:
            print(f"    {username}")

    if nonexistent_users:
        print("  Non-existent users:")
        for username in nonexistent_users:
            print(f"    {username}")

    return {
        "inactive": inactive_users,
        "active": active_users,
        "nonexistent": nonexistent_users,
    }


def main():
    """Main entry point for the script.

    Parses command-line arguments, authenticates with GitHub, and checks
    one or more repositories for inactive codeowners. Outputs a summary
    of findings.

    Returns:
        int: Exit code (0 = success, 1 = inactive users found, 2 = errors)
    """
    parser = argparse.ArgumentParser(
        description="Check for inactive codeowners in GitHub repositories"
    )
    parser.add_argument(
        "--repos",
        type=str,
        default=None,
        help="Comma-separated repository name(s) to check (if not specified, checks all repos in the organization)",
    )
    parser.add_argument(
        "--org", default="rust-vmm", help="GitHub organization (default: rust-vmm)"
    )
    parser.add_argument(
        "--days",
        type=int,
        default=365,
        help="Number of days to check for activity (default: 365)",
    )
    parser.add_argument(
        "--until",
        type=str,
        default=None,
        help="End date for activity check in YYYY-MM-DD format (default: today)",
    )
    parser.add_argument(
        "--verbose",
        "-v",
        action="store_true",
        help="Enable detailed debug output including API queries and individual activity found",
    )
    parser.add_argument(
        "--rate-limit",
        type=float,
        default=0,
        help="Delay in seconds between API requests (default: 0). "
        "PyGithub automatically handles retries and backoff.",
    )
    parser.add_argument(
        "--include-commits",
        action="store_true",
        help="Include commit count in activity check (default: only reviews and comments)",
    )

    args = parser.parse_args()

    log_level = logging.DEBUG if args.verbose else logging.WARNING

    logging.basicConfig(
        level=log_level, format="%(levelname)s: %(message)s", stream=sys.stderr
    )

    token = os.environ.get("GITHUB_TOKEN")
    if not token:
        logger.warning("GITHUB_TOKEN environment variable not set.")
        logger.warning(
            "API requests will be rate-limited. Set GITHUB_TOKEN for higher limits."
        )

    auth = Auth.Token(token) if token else None

    gh = Github(auth=auth, per_page=100, seconds_between_requests=args.rate_limit)

    # Remove PyGithub's handlers to avoid duplicate output and set level
    github_logger = logging.getLogger("github")
    github_logger.handlers.clear()
    github_logger.setLevel(logging.WARNING)

    if args.until:
        try:
            until_date = datetime.strptime(args.until, "%Y-%m-%d").replace(
                tzinfo=timezone.utc
            )
        except ValueError:
            logger.error("Invalid date format for --until. Use YYYY-MM-DD format.")
            return 2
    else:
        until_date = datetime.now(timezone.utc)

    since_date = until_date - timedelta(days=args.days)
    print(
        f"Checking activity from {since_date.date()} to {until_date.date()} ({args.days} days)"
    )

    if args.repos:
        repos_to_check = [repo.strip() for repo in args.repos.split(",")]
    else:
        print(f"Fetching all repositories from {args.org}...")
        org = gh.get_organization(args.org)
        repos_to_check = [repo.name for repo in org.get_repos()]

    repos_to_check.sort()

    print(f"{len(repos_to_check)} repositories to check: {', '.join(repos_to_check)}")

    repos_status = {}
    repos_skipped = []
    total_repos = len(repos_to_check)
    for idx, repo_name in enumerate(repos_to_check, 1):
        print(f"\nChecking {args.org}/{repo_name} ({idx}/{total_repos})...")
        try:
            status = check_repository(
                gh, args.org, repo_name, since_date, until_date, args.include_commits
            )
            if status is None:
                repos_skipped.append(repo_name)
                continue

            repos_status[repo_name] = status

        except Exception as e:
            logger.error("Failed to check repository %s: %s", repo_name, e)
            repos_skipped.append(repo_name)
            continue

    print("\n" + "=" * 80)
    print(
        f"SUMMARY - Activity from {since_date.date()} to {until_date.date()} ({args.days} days)"
    )
    print("=" * 80 + "\n")

    print(f"Total repositories: {len(repos_to_check)}")

    repos_with_issues = {}
    repos_fine = []
    for name, data in repos_status.items():
        if data["inactive"] or data["nonexistent"]:
            repos_with_issues[name] = data
        else:
            repos_fine.append(name)

    if repos_fine:
        print(f"\nRepositories with all codeowners active ({len(repos_fine)}):")
        for repo_name in repos_fine:
            print(f"  - {repo_name}")

    if repos_skipped:
        print(f"\nRepositories skipped ({len(repos_skipped)}):")
        for repo_name in repos_skipped:
            print(f"  - {repo_name}")

    if repos_with_issues:
        print(
            f"\nRepositories with inactive or non-existent codeowners ({len(repos_with_issues)}):"
        )
        for repo_name in repos_with_issues:
            issues = repos_with_issues[repo_name]
            print(f"  - {repo_name}:")
            if issues["active"]:
                active_list = ", ".join([f"@{user}" for user, _ in issues["active"]])
                print(f"      Active: {active_list}")
            if issues["inactive"]:
                inactive_list = ", ".join([f"@{user}" for user in issues["inactive"]])
                print(f"      Inactive: {inactive_list}")
            if issues["nonexistent"]:
                nonexistent_list = ", ".join(
                    [f"@{user}" for user in issues["nonexistent"]]
                )
                print(f"      Non-existent: {nonexistent_list}")

    if repos_with_issues:
        return 1

    if not repos_status:
        print("\nNo repositories were processed!")
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
