#!/usr/bin/env bash
# update-changelog.sh
#
# Scans work-reports/ for new reports and updates CHANGELOG.md.
#
# Usage:
#   ./scripts/update-changelog.sh              # dry-run: show what would be added
#   ./scripts/update-changelog.sh --apply      # actually update CHANGELOG.md
#
# Work report format expected in each file:
#   # Work Report — YYYY-MM-DD — Short Description
#   > **Version:** vX.Y.Z (or "unreleased")
#   > **Author:** ...
#   > **Type:** feat | fix | docs | refactor | chore
#   ## Summary
#   One paragraph...
#   ## Changes
#   ...

set -euo pipefail

REPORTS_DIR="work-reports"
CHANGELOG="CHANGELOG.md"
DRY_RUN=true

if [[ "${1:-}" == "--apply" ]]; then
    DRY_RUN=false
    echo ">>> APPLY mode — will update $CHANGELOG"
else
    echo ">>> DRY-RUN mode — use --apply to actually update $CHANGELOG"
fi

if [[ ! -d "$REPORTS_DIR" ]]; then
    echo "Error: $REPORTS_DIR/ not found. Create it and add work reports."
    exit 1
fi

if [[ ! -f "$CHANGELOG" ]]; then
    echo "Error: $CHANGELOG not found."
    exit 1
fi

# Extract the current top version from CHANGELOG
CURRENT_VERSION=$(grep -m1 '^## \[' "$CHANGELOG" | sed 's/## \[//;s/\] .*//')
echo "  Current top version: $CURRENT_VERSION"

# Find work reports newer than the last update
# We track processed reports in a marker file
MARKER_FILE="$REPORTS_DIR/.last-processed"
LAST_PROCESSED=""
if [[ -f "$MARKER_FILE" ]]; then
    LAST_PROCESSED=$(cat "$MARKER_FILE")
    echo "  Last processed report: $LAST_PROCESSED"
else
    echo "  No .last-processed marker found — will process all reports"
fi

# Collect new reports (sorted by date in filename)
NEW_REPORTS=()
for f in "$REPORTS_DIR"/*.md; do
    base=$(basename "$f")
    # Skip TEMPLATE.md
    [[ "$base" == "TEMPLATE.md" ]] && continue
    # Skip hidden files
    [[ "$base" == .* ]] && continue
    # Skip if already processed
    if [[ -n "$LAST_PROCESSED" ]] && [[ "$base" < "$LAST_PROCESSED" || "$base" == "$LAST_PROCESSED" ]]; then
        continue
    fi
    NEW_REPORTS+=("$base")
done

if [[ ${#NEW_REPORTS[@]} -eq 0 ]]; then
    echo "  No new reports to process."
    exit 0
fi

echo "  Found ${#NEW_REPORTS[@]} new report(s):"
for r in "${NEW_REPORTS[@]}"; do
    echo "    - $r"
done

# Build the new changelog entries
ENTRIES=""
for report_file in "${NEW_REPORTS[@]}"; do
    report_path="$REPORTS_DIR/$report_file"

    # Extract fields from the work report
    version=$(grep -m1 '^\*\*Version:\*\*' "$report_path" | sed 's/.*\*\*Version:\*\* //;s/ .*//' || echo "unreleased")
    type=$(grep -m1 '^\*\*Type:\*\*' "$report_path" | sed 's/.*\*\*Type:\*\* //;s/ .*//' || echo "chore")
    date=$(echo "$report_file" | grep -oE '^[0-9]{4}-[0-9]{2}-[0-9]{2}' || echo "")
    title=$(head -1 "$report_path" | sed 's/^# Work Report — //;s/^[0-9-]* — //' || echo "$report_file")

    # Map type to changelog section
    section=""
    case "$type" in
        feat) section="### Added" ;;
        fix)  section="### Fixed" ;;
        docs) section="### Changed" ;;
        refactor) section="### Changed" ;;
        chore) section="### Changed" ;;
        *)    section="### Changed" ;;
    esac

    ENTRIES+="- **$title** ($type)\n"
done

if $DRY_RUN; then
    echo ""
    echo "=== Would add to CHANGELOG.md ==="
    echo -e "$ENTRIES"
    echo "=== End preview ==="
    echo ""
    echo "Run with --apply to update CHANGELOG.md"
    exit 0
fi

# Build the new CHANGELOG section
NEW_SECTION="## [unreleased]\n\n$ENTRIES\n"

# Insert after the first line (the title)
TMPFILE=$(mktemp)
head -1 "$CHANGELOG" > "$TMPFILE"
echo "" >> "$TMPFILE"
echo -e "$NEW_SECTION" >> "$TMPFILE"
tail -n +2 "$CHANGELOG" >> "$TMPFILE"
mv "$TMPFILE" "$CHANGELOG"

# Update the marker
LATEST=$(printf '%s\n' "${NEW_REPORTS[@]}" | sort | tail -1)
echo "$LATEST" > "$MARKER_FILE"

echo "  Updated $CHANGELOG with ${#NEW_REPORTS[@]} new entries."
echo "  Marker set to: $LATEST"
echo ""
echo "  Next: review $CHANGELOG, then commit:"
echo "    git add $CHANGELOG $MARKER_FILE && git commit -m 'docs: update changelog from work reports'"
