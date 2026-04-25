#!/usr/bin/env bash
# Configure main branch protection per R5.5 of docs/specs/dwarven.md.
# Requires admin privileges on the target repo.

set -euo pipefail

REPO=$(gh repo view --json nameWithOwner --jq .nameWithOwner)

echo "Configuring branch protection on $REPO main..."

gh api -X PUT "repos/$REPO/branches/main/protection" --input - <<'EOF'
{
  "required_status_checks": null,
  "enforce_admins": false,
  "required_pull_request_reviews": {
    "required_approving_review_count": 1
  },
  "restrictions": null,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF

echo "Branch protection configured: PR review required (1 approval), no force-push, no deletion."
echo "Note: Spec, Architect, Implementation Planning, and System Documentation agents commit directly to main per R5.5.1 — they bypass this protection by design (their tool allowlists permit git push origin main:*)."
