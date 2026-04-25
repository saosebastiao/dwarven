#!/usr/bin/env bash
# Create the Dwarven v0.1 label set in the current GitHub repo.
# Idempotent: existing labels get color updates; missing labels are created.
# Requires gh authenticated and write access to the target repo.

set -euo pipefail

# agent:* (purple)
AGENT_LABELS=(
    "agent:spec"
    "agent:architect"
    "agent:gap"
    "agent:pm"
    "agent:plan"
    "agent:test"
    "agent:implement"
    "agent:review"
    "agent:doc"
    "agent:triage"
    "agent:maintainer"
)
AGENT_COLOR="6f42c1"

# type:* (blue)
TYPE_LABELS=(
    "type:spec-gap"
    "type:feature"
    "type:bug"
    "type:arch"
    "type:doc"
    "type:chore"
)
TYPE_COLOR="0075ca"

# blocker:* (red)
BLOCKER_LABELS=(
    "blocker:maintainer-input"
    "blocker:external"
    "blocker:upstream"
)
BLOCKER_COLOR="b60205"

# priority:* (graduated red→yellow→grey)
PRIORITY_NAMES=(
    "priority:p0"
    "priority:p1"
    "priority:p2"
)
PRIORITY_COLORS=(
    "d93f0b"
    "fbca04"
    "cccccc"
)

create_or_update() {
    local name="$1"
    local color="$2"
    if gh label list --json name --jq '.[].name' | grep -qFx "$name"; then
        gh label edit "$name" --color "$color" >/dev/null
        echo "  updated: $name"
    else
        gh label create "$name" --color "$color" >/dev/null
        echo "  created: $name"
    fi
}

echo "Creating agent:* labels..."
for label in "${AGENT_LABELS[@]}"; do
    create_or_update "$label" "$AGENT_COLOR"
done

echo "Creating type:* labels..."
for label in "${TYPE_LABELS[@]}"; do
    create_or_update "$label" "$TYPE_COLOR"
done

echo "Creating blocker:* labels..."
for label in "${BLOCKER_LABELS[@]}"; do
    create_or_update "$label" "$BLOCKER_COLOR"
done

echo "Creating priority:* labels..."
for i in "${!PRIORITY_NAMES[@]}"; do
    create_or_update "${PRIORITY_NAMES[$i]}" "${PRIORITY_COLORS[$i]}"
done

echo "Done. epic:<slug> labels are created on demand by PM (R3.4)."
