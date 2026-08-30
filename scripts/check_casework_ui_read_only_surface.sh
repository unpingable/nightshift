#!/usr/bin/env bash
# Structural qualification gate for the Nightshift Casework browser surface.

set -uo pipefail

repo_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd)
cd "$repo_root"

ui_root="ui/casework"
app_source="$ui_root/src/App.tsx"
findings=0

fail() {
    printf 'casework-ui-read-only: %s\n' "$1" >&2
    findings=$((findings + 1))
}

if [ ! -f "$ui_root/package-lock.json" ] || [ ! -f "$app_source" ]; then
    fail "frontend lockfile or application source is missing"
fi

if rg -n -i '(https?://|//fonts\.|analytics|telemetry)' \
    "$ui_root/index.html" "$ui_root/src" \
    -g '!*.test.ts' -g '!*.test.tsx' -g '!**/test/**' \
    >/tmp/nightshift_casework_ui_hits 2>/dev/null; then
    fail "remote runtime asset or observation endpoint found:"
    cat /tmp/nightshift_casework_ui_hits >&2
fi

if rg -n -i "<(button|textarea)|contentEditable|type=['\"](submit|button)['\"]" \
    "$ui_root/src" -g '!*.test.ts' -g '!*.test.tsx' -g '!**/test/**' \
    >/tmp/nightshift_casework_ui_hits 2>/dev/null; then
    fail "operator mutation-capable control element found:"
    cat /tmp/nightshift_casework_ui_hits >&2
fi

if rg -n -i "method:[[:space:]]*['\"](POST|PUT|PATCH|DELETE)['\"]" \
    "$ui_root/src" -g '!*.test.ts' -g '!*.test.tsx' -g '!**/test/**' \
    >/tmp/nightshift_casework_ui_hits 2>/dev/null; then
    fail "browser write method found:"
    cat /tmp/nightshift_casework_ui_hits >&2
fi

if ! rg -q 'method: "GET"' "$ui_root/src/api.ts"; then
    fail "API adapter does not pin requests to GET"
fi
if ! rg -q 'Skip to casework' "$app_source" \
    || ! rg -q ':focus-visible' "$ui_root/src/styles.css"; then
    fail "keyboard skip or visible-focus treatment is missing"
fi
if ! rg -q -- '--faint: #84919c;' "$ui_root/src/styles.css"; then
    fail "informative faint text is not pinned to the qualified contrast color"
fi

if [ "$#" -gt 0 ] && [ "$1" = "--self-test-inject" ]; then
    sandbox=$(mktemp -d)
    trap 'rm -rf "$sandbox" /tmp/nightshift_casework_ui_hits /tmp/nightshift_casework_ui_selftest' EXIT
    mkdir -p "$sandbox/repo/ui/casework/src"
    cp -a scripts "$sandbox/repo/"
    cp "$ui_root/package-lock.json" "$ui_root/index.html" "$sandbox/repo/ui/casework/"
    cp -a "$ui_root/src/." "$sandbox/repo/ui/casework/src/"
    printf '\nexport const injectedWriteControl = <button>Dispatch case</button>;\n' \
        >>"$sandbox/repo/ui/casework/src/App.tsx"
    if (cd "$sandbox/repo" && bash scripts/check_casework_ui_read_only_surface.sh \
        >/dev/null 2>/tmp/nightshift_casework_ui_selftest); then
        printf 'self-test FAILED: injected operator control passed\n' >&2
        exit 1
    fi
    if ! rg -q 'mutation-capable control' /tmp/nightshift_casework_ui_selftest; then
        printf 'self-test FAILED: injected control was not reported\n' >&2
        cat /tmp/nightshift_casework_ui_selftest >&2
        exit 1
    fi
    printf 'self-test PASSED: injected operator control was refused.\n' >&2
    exit 0
fi

rm -f /tmp/nightshift_casework_ui_hits
if [ "$findings" -ne 0 ]; then
    printf 'casework-ui-read-only: FAILED (%d finding(s))\n' "$findings" >&2
    exit 1
fi
printf 'casework-ui-read-only: clean read-only browser surface\n'
