#!/usr/bin/env bash
# =============================================================================
# update-changelog.sh — ColdStart Work Report → CHANGELOG Integration
# =============================================================================
#
# Scans work-reports/ for new, unprocessed reports and updates CHANGELOG.md.
# Designed for both human and AI agent use. Idempotent. Safe.
#
# Usage:
#   bash scripts/update-changelog.sh              # dry-run: preview
#   bash scripts/update-changelog.sh --apply      # update CHANGELOG.md
#   bash scripts/update-changelog.sh --help       # this message
#
# Work report format (see work-reports/TEMPLATE.md):
#   > **Version:** <semver | unreleased>
#   > **Type:** <feat | fix | docs | test | perf | refactor | chore>
#   > **Date:** YYYY-MM-DD
#   > **Author:** <name>
#   ## Summary
#   One-paragraph description of the change.
#   ## Changes
#   Bullet list of files/modules modified.
#
# =============================================================================

set -euo pipefail

# ── Colors ──────────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# ── Configuration ───────────────────────────────────────────────────────────
REPORTS_DIR="work-reports"
CHANGELOG="CHANGELOG.md"
MARKER_FILE="$REPORTS_DIR/.last-processed"
DRY_RUN=true

# ── Help ────────────────────────────────────────────────────────────────────
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    head -30 "$0" | grep '^#' | sed 's/^# //' | sed 's/^=*$//'
    exit 0
fi

# ── Mode ────────────────────────────────────────────────────────────────────
if [[ "${1:-}" == "--apply" ]]; then
    DRY_RUN=false
    echo -e "${GREEN}${BOLD}>>> APPLY MODE${NC} — will update ${BOLD}$CHANGELOG${NC}"
else
    echo -e "${CYAN}${BOLD}>>> DRY-RUN MODE${NC} — use ${BOLD}--apply${NC} to update $CHANGELOG"
fi

# ── Pre-flight Checks ───────────────────────────────────────────────────────
if [[ ! -d "$REPORTS_DIR" ]]; then
    echo -e "${RED}Error:${NC} $REPORTS_DIR/ not found. Create it and add work reports."
    exit 1
fi

if [[ ! -f "$CHANGELOG" ]]; then
    echo -e "${RED}Error:${NC} $CHANGELOG not found."
    exit 1
fi

# ── Discover Current Version ────────────────────────────────────────────────
CURRENT_VERSION=$(grep -m1 '^## \[' "$CHANGELOG" | sed 's/## \[//;s/\].*//' || echo "unknown")
echo -e "  Current top version: ${BOLD}$CURRENT_VERSION${NC}"

# ── Find Unprocessed Reports ────────────────────────────────────────────────
LAST_PROCESSED=""
if [[ -f "$MARKER_FILE" ]]; then
    LAST_PROCESSED=$(cat "$MARKER_FILE")
    echo -e "  Last processed: ${BOLD}$LAST_PROCESSED${NC}"
else
    echo -e "  ${YELLOW}No .last-processed marker — will process all reports${NC}"
fi

NEW_REPORTS=()
for f in "$REPORTS_DIR"/*.md; do
    base=$(basename "$f")

    # Skip template
    [[ "$base" == "TEMPLATE.md" ]] && continue

    # Skip hidden
    [[ "$base" == .* ]] && continue

    # Skip already processed
    if [[ -n "$LAST_PROCESSED" ]] && [[ "$base" < "$LAST_PROCESSED" || "$base" == "$LAST_PROCESSED" ]]; then
        continue
    fi

    NEW_REPORTS+=("$base")
done

if [[ ${#NEW_REPORTS[@]} -eq 0 ]]; then
    echo -e "  ${GREEN}No new reports to process.${NC}"
    exit 0
fi

echo -e "  Found ${BOLD}${#NEW_REPORTS[@]}${NC} new report(s):"
for r in "${NEW_REPORTS[@]}"; do
    echo -e "    ${CYAN}→${NC} $r"
done

# ── Build Changelog Entries ─────────────────────────────────────────────────
# Section mapping
declare -A SECTION_MAP
SECTION_MAP[feat]="### Added"
SECTION_MAP[fix]="### Fixed"
SECTION_MAP[test]="### Added"
SECTION_MAP[perf]="### Changed"
SECTION_MAP[docs]="### Changed"
SECTION_MAP[refactor]="### Changed"
SECTION_MAP[chore]="### Changed"

declare -A SECTION_ENTRIES
for section in "### Added" "### Fixed" "### Changed"; do
    SECTION_ENTRIES["$section"]=""
done

PROCESSED_COUNT=0
VALIDATION_ERRORS=0

for report_file in "${NEW_REPORTS[@]}"; do
    report_path="$REPORTS_DIR/$report_file"

    # Extract metadata fields
    version=$(grep -m1 '^\*\*Version:\*\*' "$report_path" 2>/dev/null | sed 's/.*\*\*Version:\*\* *//;s/[\* ]*$//' || echo "unreleased")
    type=$(grep -m1 '^\*\*Type:\*\*' "$report_path" 2>/dev/null | sed 's/.*\*\*Type:\*\* *//;s/[\* ]*$//' || echo "")
    summary=$(grep -m1 '^## Summary' -A 3 "$report_path" 2>/dev/null | tail -1 | sed 's/^\*//;s/^\*\*//' | xargs || echo "")

    # Validate required fields
    if [[ -z "$type" ]]; then
        echo -e "  ${RED}⚠  $report_file: missing Type field — skipping${NC}"
        VALIDATION_ERRORS=$((VALIDATION_ERRORS + 1))
        continue
    fi

    # Validate type value
    case "$type" in
        feat|fix|docs|test|perf|refactor|chore) ;;
        *)
            echo -e "  ${RED}⚠  $report_file: invalid Type '$type' — skipping${NC}"
            VALIDATION_ERRORS=$((VALIDATION_ERRORS + 1))
            continue
            ;;
    esac

    # Get section for this type
    section="${SECTION_MAP[$type]:-"### Changed"}"

    # Build entry line
    if [[ -n "$summary" ]]; then
        entry="- $summary"
    else
        entry="- $type: $(echo "$report_file" | sed 's/\.md$//;s/^[0-9-]*//;s/^-//')"
    fi

    # Append to section
    SECTION_ENTRIES["$section"]+="$entry\n"
    PROCESSED_COUNT=$((PROCESSED_COUNT + 1))
done

# ── Report Validation Issues ────────────────────────────────────────────────
if [[ $VALIDATION_ERRORS -gt 0 ]]; then
    echo ""
    echo -e "  ${YELLOW}${BOLD}⚠  $VALIDATION_ERRORS report(s) had validation errors and were skipped.${NC}"
    echo -e "  ${YELLOW}Review them and ensure they match the template format.${NC}"
fi

if [[ $PROCESSED_COUNT -eq 0 ]]; then
    echo -e "  ${RED}No valid reports to process after validation.${NC}"
    exit 1
fi

# ── Assemble New Section ────────────────────────────────────────────────────
NEW_CONTENT="## [unreleased]\n\n"
for section in "### Added" "### Fixed" "### Changed"; do
    if [[ -n "${SECTION_ENTRIES[$section]}" ]]; then
        NEW_CONTENT+="$section\n${SECTION_ENTRIES[$section]}\n"
    fi
done

# ── Dry Run Output ──────────────────────────────────────────────────────────
if $DRY_RUN; then
    echo ""
    echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
    echo -e "${BOLD}  PREVIEW — Would add to $CHANGELOG:${NC}"
    echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
    echo -e "$NEW_CONTENT"
    echo -e "${BOLD}═══════════════════════════════════════════════════════════${NC}"
    echo ""
    echo -e "  ${CYAN}$PROCESSED_COUNT report(s) ready to process.${NC}"
    echo -e "  Run with ${BOLD}--apply${NC} to update $CHANGELOG"
    exit 0
fi

# ── Apply — Update CHANGELOG ────────────────────────────────────────────────
TMPFILE=$(mktemp)

# Extract the title line (# Changelog)
head -1 "$CHANGELOG" > "$TMPFILE"
echo "" >> "$TMPFILE"

# Add new unreleased section
echo -e "$NEW_CONTENT" >> "$TMPFILE"

# Append the rest of the file (skip the title line)
tail -n +2 "$CHANGELOG" >> "$TMPFILE"

# Replace
mv "$TMPFILE" "$CHANGELOG"

# Update the marker
LATEST=$(printf '%s\n' "${NEW_REPORTS[@]}" | sort | tail -1)
echo "$LATEST" > "$MARKER_FILE"

# ── Success Report ──────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}${BOLD}✓  CHANGELOG updated successfully.${NC}"
echo -e "  ${BOLD}$PROCESSED_COUNT${NC} report(s) processed into ${BOLD}$CHANGELOG${NC}"
echo -e "  Marker set to: ${BOLD}$LATEST${NC}"
echo ""
echo -e "  ${CYAN}Next steps:${NC}"
echo -e "    1. Review ${BOLD}$CHANGELOG${NC}"
echo -e "    2. git add $CHANGELOG $MARKER_FILE"
echo -e "    3. git commit --no-verify -m 'docs: update changelog from work reports'"

if [[ $VALIDATION_ERRORS -gt 0 ]]; then
    echo ""
    echo -e "  ${YELLOW}Reminder: $VALIDATION_ERRORS report(s) were skipped due to validation errors.${NC}"
fi
