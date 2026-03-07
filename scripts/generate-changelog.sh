#!/usr/bin/env bash
#
# generate-changelog.sh - Generate changelog from git commits
#
# Usage:
#   ./scripts/generate-changelog.sh [FROM_TAG] [TO_TAG]
#   ./scripts/generate-changelog.sh              # Since last tag
#   ./scripts/generate-changelog.sh v0.1.0       # Since v0.1.0
#   ./scripts/generate-changelog.sh v0.1.0 v0.2.0 # Between two tags
#
# This script parses conventional commits and generates a changelog section.
# It follows the Keep a Changelog format.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
CHANGELOG_FILE="$PROJECT_ROOT/CHANGELOG.md"

# Commit type to changelog section mapping
declare -A TYPE_MAP=(
    ["feat"]="Added"
    ["feature"]="Added"
    ["change"]="Changed"
    ["changed"]="Changed"
    ["deprecate"]="Deprecated"
    ["deprecated"]="Deprecated"
    ["remove"]="Removed"
    ["removed"]="Removed"
    ["fix"]="Fixed"
    ["bugfix"]="Fixed"
    ["security"]="Security"
    ["docs"]="Changed"
    ["documentation"]="Changed"
)

# Scopes that are considered valid
VALID_SCOPES=(
    "cli" "config" "agent" "pipeline" "tasks" "git"
    "memory" "logging" "progress" "testing" "telemetry"
    "mcp" "interactive" "output" "man" "completions"
    "workspace" "validate" "dryrun" "feature"
)

# Get git revision range
get_revision_range() {
    local from_tag="${1:-}"
    local to_tag="${2:-HEAD}"

    if [[ -z "$from_tag" ]]; then
        # Get the most recent tag
        from_tag=$(git describe --tags --abbrev=0 2>/dev/null || echo "")
        if [[ -z "$from_tag" ]]; then
            # No tags exist, use initial commit
            from_tag=$(git rev-list --max-parents=0 HEAD)
        fi
    fi

    echo "${from_tag}..${to_tag}"
}

# Parse conventional commit message
# Returns: type|scope|description|breaking
parse_commit() {
    local message="$1"

    # Pattern: type(scope)!: description or type!: description
    if [[ "$message" =~ ^([a-z]+)(\(([a-z-]+)\))?(\!)?:\ (.+)$ ]]; then
        local type="${BASH_REMATCH[1]}"
        local scope="${BASH_REMATCH[3]:-}"
        local breaking="${BASH_REMATCH[4]:-}"
        local description="${BASH_REMATCH[5]}"

        echo "${type}|${scope}|${description}|${breaking}"
        return 0
    fi

    return 1
}

# Check if scope is valid
is_valid_scope() {
    local scope="$1"

    if [[ -z "$scope" ]]; then
        return 0
    fi

    for valid_scope in "${VALID_SCOPES[@]}"; do
        if [[ "$scope" == "$valid_scope" ]]; then
            return 0
        fi
    done

    return 1
}

# Get changelog section for commit type
get_section() {
    local type="$1"

    # Check for breaking changes first
    if [[ "$type" == *"!"* ]]; then
        echo "Breaking Changes"
        return
    fi

    local section="${TYPE_MAP[$type]:-}"
    if [[ -n "$section" ]]; then
        echo "$section"
    else
        echo "Changed"
    fi
}

# Format scope for display
format_scope() {
    local scope="$1"

    if [[ -z "$scope" ]]; then
        echo ""
        return
    fi

    # Capitalize first letter
    echo "**${scope^}**: "
}

# Get commits between revisions
get_commits() {
    local range="$1"

    git log --pretty=format:"%H|%s" "$range" 2>/dev/null
}

# Generate changelog entries
generate_changelog() {
    local range="$1"

    # Temporary files for collecting entries
    local tmp_dir
    tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    # Initialize section files
    for section in "Added" "Changed" "Deprecated" "Removed" "Fixed" "Security" "Breaking Changes"; do
        touch "$tmp_dir/$section"
    done

    # Process commits
    while IFS='|' read -r hash subject; do
        # Skip empty lines
        [[ -z "$hash" ]] && continue

        # Skip merge commits
        if [[ "$subject" =~ ^Merge\ (branch|pull\ request) ]]; then
            continue
        fi

        # Parse commit message
        local parsed
        if ! parsed=$(parse_commit "$subject"); then
            # Non-conventional commit, skip or add to "Changed"
            continue
        fi

        IFS='|' read -r type scope description breaking <<< "$parsed"

        # Skip certain commit types (internal changes)
        if [[ "$type" =~ ^(refactor|test|chore|style|ci|build|perf)$ ]]; then
            continue
        fi

        # Get section
        local section
        if [[ -n "$breaking" ]]; then
            section="Breaking Changes"
        else
            section=$(get_section "$type")
        fi

        # Format entry
        local scope_formatted
        scope_formatted=$(format_scope "$scope")

        # Get PR number if available
        local pr_link=""
        local pr_number
        pr_number=$(git log --pretty=format:"%b" "$hash" -1 | grep -oP '#\K[0-9]+' | head -1 || true)
        if [[ -n "$pr_number" ]]; then
            pr_link=" (#$pr_number)"
        fi

        # Add entry to section file
        echo "- ${scope_formatted}${description}${pr_link}" >> "$tmp_dir/$section"
    done < <(get_commits "$range")

    # Output changelog
    local output=""

    # Output sections in order
    for section in "Breaking Changes" "Added" "Changed" "Deprecated" "Removed" "Fixed" "Security"; do
        if [[ -s "$tmp_dir/$section" ]]; then
            # Sort and deduplicate
            local entries
            entries=$(sort -u "$tmp_dir/$section")

            # Count entries
            local count
            count=$(echo "$entries" | wc -l)

            if [[ $count -gt 0 ]]; then
                output+="\n### $section\n\n"

                # Group by scope if there are many entries
                if [[ $count -gt 10 ]]; then
                    # Group by scope
                    declare -A scope_entries
                    local no_scope_entries=""

                    while IFS= read -r entry; do
                        if [[ "$entry" =~ ^-\ \*\*([A-Za-z]+)\*\*:\ (.+)$ ]]; then
                            local entry_scope="${BASH_REMATCH[1]}"
                            local entry_desc="${BASH_REMATCH[2]}"
                            scope_entries["$entry_scope"]+="  - $entry_desc\n"
                        else
                            no_scope_entries+="  $entry\n"
                        fi
                    done <<< "$entries"

                    # Output scoped entries
                    for scope in "${!scope_entries[@]}"; do
                        output+="#### ${scope^}\n"
                        output+="$(echo -e "${scope_entries[$scope]}")\n"
                    done

                    # Output non-scoped entries
                    if [[ -n "$no_scope_entries" ]]; then
                        output+="#### General\n"
                        output+="$(echo -e "$no_scope_entries")\n"
                    fi
                else
                    # Output flat list
                    while IFS= read -r entry; do
                        output+="  $entry\n"
                    done <<< "$entries"
                fi

                output+="\n"
            fi
        fi
    done

    echo -e "$output"
}

# Update CHANGELOG.md
update_changelog() {
    local version="$1"
    local date="$2"
    local content="$3"

    # Check if version section already exists
    if grep -q "## \[$version\]" "$CHANGELOG_FILE"; then
        echo -e "${YELLOW}Version $version already exists in CHANGELOG.md${NC}"
        return 1
    fi

    # Create backup
    cp "$CHANGELOG_FILE" "$CHANGELOG_FILE.bak"

    # Find the Unreleased section and insert new version
    local unreleased_pattern="## \[Unreleased\]"

    # Check if Unreleased section exists
    if grep -q "$unreleased_pattern" "$CHANGELOG_FILE"; then
        # Insert new version after Unreleased section
        local new_section="## [$version] - $date\n\n$content"

        # Use sed to insert after Unreleased section
        # First, move Unreleased content to the new version
        # Then clear Unreleased

        # Create temp file with new structure
        awk -v version="$version" -v date="$date" -v content="$content" '
        /^## \[Unreleased\]/ {
            print $0
            print ""
            # Skip to next section, printing unreleased content
            while ((getline line) > 0 && line !~ /^## /) {
                # Skip - we want to move to new version
            }
            # Print new version section
            print "## [" version "] - " date
            print ""
            print content
            print ""
            # Print the line we just read (start of next section)
            if (line ~ /^## /) print line
            next
        }
        { print }
        ' "$CHANGELOG_FILE" > "$CHANGELOG_FILE.tmp"

        mv "$CHANGELOG_FILE.tmp" "$CHANGELOG_FILE"
    else
        echo -e "${RED}No [Unreleased] section found in CHANGELOG.md${NC}"
        return 1
    fi

    rm -f "$CHANGELOG_FILE.bak"
    echo -e "${GREEN}Updated CHANGELOG.md with version $version${NC}"
}

# Print usage
usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [FROM_TAG] [TO_TAG]

Generate changelog from git commits using conventional commit format.

Arguments:
  FROM_TAG    Starting tag (default: last tag)
  TO_TAG      Ending tag (default: HEAD)

Options:
  -h, --help              Show this help message
  -v, --version VERSION   Specify version for new release
  -d, --date DATE         Specify release date (default: today)
  -u, --update            Update CHANGELOG.md
  -o, --output FILE       Write output to file
  --dry-run               Show what would be done without making changes

Examples:
  $(basename "$0")                              # Generate since last tag
  $(basename "$0") v0.1.0                       # Generate since v0.1.0
  $(basename "$0") v0.1.0 v0.2.0               # Generate between tags
  $(basename "$0") -v 0.2.0 -u                  # Update CHANGELOG.md
  $(basename "$0") --dry-run -v 0.2.0 -u        # Preview changes

Conventional Commit Format:
  type(scope): description
  type!: description                    # Breaking change
  feat(cli): add --parallel flag
  fix(pipeline): handle empty tasks
  docs: update README

Commit Types:
  feat/feature     → Added
  fix/bugfix       → Fixed
  change           → Changed
  deprecate        → Deprecated
  remove           → Removed
  security         → Security
  docs             → Changed
  refactor/test/chore/style/ci/build/perf → (not included)
EOF
}

# Main function
main() {
    local version=""
    local date=""
    local update=false
    local output_file=""
    local dry_run=false
    local from_tag=""
    local to_tag="HEAD"

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case "$1" in
            -h|--help)
                usage
                exit 0
                ;;
            -v|--version)
                version="$2"
                shift 2
                ;;
            -d|--date)
                date="$2"
                shift 2
                ;;
            -u|--update)
                update=true
                shift
                ;;
            -o|--output)
                output_file="$2"
                shift 2
                ;;
            --dry-run)
                dry_run=true
                shift
                ;;
            -*)
                echo -e "${RED}Unknown option: $1${NC}" >&2
                usage >&2
                exit 1
                ;;
            *)
                if [[ -z "$from_tag" ]]; then
                    from_tag="$1"
                elif [[ "$to_tag" == "HEAD" ]]; then
                    to_tag="$1"
                else
                    echo -e "${RED}Too many arguments${NC}" >&2
                    usage >&2
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Set default date
    if [[ -z "$date" ]]; then
        date=$(date +%Y-%m-%d)
    fi

    # Get revision range
    local range
    range=$(get_revision_range "$from_tag" "$to_tag")

    echo -e "${BLUE}Generating changelog for range: $range${NC}" >&2

    # Generate changelog content
    local content
    content=$(generate_changelog "$range")

    if [[ -z "$content" ]]; then
        echo -e "${YELLOW}No conventional commits found in range${NC}" >&2
        exit 0
    fi

    # Output
    if [[ -n "$output_file" ]]; then
        echo -e "$content" > "$output_file"
        echo -e "${GREEN}Changelog written to $output_file${NC}" >&2
    elif [[ "$update" == true ]]; then
        if [[ -z "$version" ]]; then
            echo -e "${RED}Version required for update. Use -v VERSION${NC}" >&2
            exit 1
        fi

        if [[ "$dry_run" == true ]]; then
            echo -e "${BLUE}Would update CHANGELOG.md with:${NC}"
            echo -e "\n## [$version] - $date\n\n$content"
        else
            update_changelog "$version" "$date" "$content"
        fi
    else
        echo -e "$content"
    fi
}

# Run main
main "$@"