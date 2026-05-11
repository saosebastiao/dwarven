---
id: 15
title: 'docs/http-api-reference.md: endpoint catalog with examples'
type: doc
state: doc
priority: p1
epic: user-docs
created: 2026-05-11T05:16:39Z
created_by: maintainer
updated: 2026-05-11T05:16:39Z
---
User-facing HTTP API reference. The spec (web-api.md) defines the contract; this doc gives endpoint-by-endpoint examples with realistic request/response payloads (curl-style).

Scope:
- Base URL + auth model (none in v1; local-only)
- Error envelope shape with status code mapping
- Endpoint catalog:
  * GET /api/v1/daemon
  * GET/POST /api/v1/issues + query params for filtering
  * GET/PATCH /api/v1/issues/:id
  * GET/POST /api/v1/issues/:id/comments + ?kind filter
  * POST /api/v1/issues/:id/transitions (including to=done|dropped close routing)
  * PUT/DELETE /api/v1/issues/:id/blocker
  * PUT/DELETE /api/v1/issues/:id/priority
  * GET/POST /api/v1/dependencies, GET /api/v1/issues/:id/dependencies, DELETE /api/v1/dependencies/:from/:to
  * GET /api/v1/scheduler/queue, POST /api/v1/scheduler/override
  * POST /api/v1/daemon/{shutdown,reindex}
  * GET/PATCH /api/v1/config
  * GET /api/v1/events (SSE; Last-Event-ID protocol)
- For each endpoint: method + path + request shape + response shape + 2 concrete curl examples (success + error).
- SSE event vocabulary table (issue.created, etc.) with payload examples.

Audience: someone integrating with the local daemon (script, custom UI, CI tooling).
