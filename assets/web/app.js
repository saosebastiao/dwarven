// Dwarven web UI (v1, minimum-viable). Vanilla JS, no framework.
// See docs/specs/web-ui.md for the surface contract.

const API = "/api/v1";

const state = {
    // Cache of the most recent /issues fetch; refreshed on SSE events.
    issues: [],
    // Connection state for the SSE stream.
    sseConnected: false,
};

const app = document.getElementById("app");
const connStatus = document.getElementById("connection-status");
const inboxBadge = document.getElementById("inbox-badge");

function setConn(state) {
    if (state === "connected") {
        connStatus.textContent = "connected";
        connStatus.className = "status connected";
    } else if (state === "disconnected") {
        connStatus.textContent = "disconnected";
        connStatus.className = "status disconnected";
    } else {
        connStatus.textContent = "connecting…";
        connStatus.className = "status";
    }
}

function escapeHtml(s) {
    if (s == null) return "";
    return String(s)
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}

async function getJSON(path) {
    const resp = await fetch(`${API}${path}`);
    if (!resp.ok) throw new Error(`${resp.status} ${resp.statusText}`);
    return resp.json();
}

async function postJSON(path, body) {
    return fetchJSON("POST", path, body);
}

async function patchJSON(path, body) {
    return fetchJSON("PATCH", path, body);
}

async function putJSON(path, body) {
    return fetchJSON("PUT", path, body);
}

async function deleteJSON(path, body) {
    return fetchJSON("DELETE", path, body);
}

async function fetchJSON(method, path, body) {
    const init = {
        method,
        headers: { "Content-Type": "application/json" },
    };
    if (body !== undefined) init.body = JSON.stringify(body);
    const resp = await fetch(`${API}${path}`, init);
    if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err.message || `${resp.status} ${resp.statusText}`);
    }
    if (resp.status === 204) return null;
    return resp.json();
}

// ---------- Routing ----------

function parseHash() {
    const raw = window.location.hash.replace(/^#\/?/, "");
    const [pathPart, queryPart = ""] = raw.split("?");
    const params = new URLSearchParams(queryPart);
    if (!pathPart) return { route: "inbox", params };
    const parts = pathPart.split("/");
    if (parts[0] === "issues" && parts[1]) {
        return { route: "issue", id: parts[1], params };
    }
    return { route: parts[0], params };
}

function setHash(route, params) {
    let h = `#/${route}`;
    if (params instanceof URLSearchParams) {
        const s = params.toString();
        if (s) h += `?${s}`;
    }
    if (window.location.hash !== h) {
        window.location.hash = h;
    } else {
        // Same hash: re-render manually since hashchange won't fire.
        render();
    }
}

async function render() {
    const r = parseHash();
    document
        .querySelectorAll("nav a")
        .forEach((a) => a.classList.toggle("active", a.dataset.route === r.route || (a.dataset.route === "issues" && r.route === "issue")));
    if (r.route === "inbox") return renderInbox();
    if (r.route === "issues") return renderIssues(r.params);
    if (r.route === "issue") return renderIssue(r.id);
    if (r.route === "deps") return renderDeps(r.params);
    if (r.route === "schedule") return renderSchedule();
    if (r.route === "daemon") return renderDaemon();
    if (r.route === "config") return renderConfig();
    app.innerHTML = `<div class="empty">Unknown route. <a href="#/inbox">Back to Inbox</a></div>`;
}

window.addEventListener("hashchange", render);

// ---------- Inbox ----------

async function loadInbox() {
    // R4.1: state=maintainer OR any blocker set.
    const all = await getJSON("/issues?all=true");
    return all.filter(
        (i) => i.state === "maintainer" || (i.blocker && i.blocker.length > 0),
    );
}

async function renderInbox() {
    app.innerHTML = `<div class="empty">loading…</div>`;
    try {
        const items = await loadInbox();
        inboxBadge.textContent = items.length > 0 ? String(items.length) : "";
        if (items.length === 0) {
            app.innerHTML = `<div class="empty">Nothing waiting on you.</div>`;
            return;
        }
        app.innerHTML = `
            <h2>Inbox</h2>
            <table>
                <thead><tr>
                    <th>ID</th><th>Title</th><th>State</th>
                    <th>Blocker</th><th>Priority</th><th>Updated</th>
                </tr></thead>
                <tbody>${items
                    .map(
                        (i) => `
                    <tr class="row" data-id="${i.id}">
                        <td>#${i.id}</td>
                        <td>${escapeHtml(i.title)}</td>
                        <td><span class="tag">${i.state}</span></td>
                        <td>${i.blocker ? `<span class="tag">${i.blocker}</span>` : ""}</td>
                        <td>${i.priority ? `<span class="tag priority-${i.priority}">${i.priority}</span>` : ""}</td>
                        <td>${escapeHtml(i.updated)}</td>
                    </tr>`,
                    )
                    .join("")}</tbody>
            </table>`;
        wireRowClicks();
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

// ---------- Issues list ----------

async function renderIssues(params) {
    app.innerHTML = `<div class="empty">loading…</div>`;
    try {
        const qs = params.toString() ? `?${params.toString()}` : "";
        const items = await getJSON(`/issues${qs}`);
        state.issues = items;
        const stateField = params.get("state") ?? "";
        const typeField = params.get("type") ?? "";
        const priorityField = params.get("priority") ?? "";
        const epicField = params.get("epic") ?? "";
        const grepField = params.get("grep") ?? "";
        const scope = params.get("all") === "true"
            ? "all"
            : params.get("closed") === "true"
                ? "closed"
                : "open";

        const filterBar = `
            <details ${qs ? "open" : ""}>
                <summary>Filter${qs ? ` (${params.toString()})` : ""}</summary>
                <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 0.5rem;">
                    <div><label>state (csv)</label><input id="f-state" value="${escapeHtml(stateField)}" placeholder="plan,test"></div>
                    <div><label>type (csv)</label><input id="f-type" value="${escapeHtml(typeField)}" placeholder="feature,bug"></div>
                    <div><label>priority (csv; or 'unset')</label><input id="f-priority" value="${escapeHtml(priorityField)}" placeholder="p0,p1"></div>
                    <div><label>epic</label><input id="f-epic" value="${escapeHtml(epicField)}"></div>
                    <div style="grid-column: span 2;"><label>grep title + body</label><input id="f-grep" value="${escapeHtml(grepField)}" placeholder="literal substring"></div>
                </div>
                <div class="toolbar">
                    <label><input type="radio" name="scope" value="open" ${scope === "open" ? "checked" : ""}> open (active states)</label>
                    <label><input type="radio" name="scope" value="closed" ${scope === "closed" ? "checked" : ""}> closed (terminal only)</label>
                    <label><input type="radio" name="scope" value="all" ${scope === "all" ? "checked" : ""}> all</label>
                </div>
                <div class="toolbar">
                    <button id="f-apply">Apply filter</button>
                    <button id="f-clear">Clear</button>
                </div>
            </details>`;

        if (items.length === 0) {
            app.innerHTML = `<h2>Issues</h2>${filterBar}<div class="empty">No issues match.</div>`;
            wireFilterControls();
            return;
        }
        app.innerHTML = `
            <h2>Issues (${items.length})</h2>
            ${filterBar}
            <table>
                <thead><tr>
                    <th>ID</th><th>Title</th><th>State</th>
                    <th>Type</th><th>Priority</th><th>Updated</th>
                </tr></thead>
                <tbody>${items
                    .map(
                        (i) => `
                    <tr class="row" data-id="${i.id}">
                        <td>#${i.id}</td>
                        <td>${escapeHtml(i.title)}</td>
                        <td><span class="tag">${i.state}</span></td>
                        <td><span class="tag">${i.type}</span></td>
                        <td>${i.priority ? `<span class="tag priority-${i.priority}">${i.priority}</span>` : ""}</td>
                        <td>${escapeHtml(i.updated)}</td>
                    </tr>`,
                    )
                    .join("")}</tbody>
            </table>`;
        wireRowClicks();
        wireFilterControls();
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

function wireFilterControls() {
    const apply = document.getElementById("f-apply");
    const clear = document.getElementById("f-clear");
    if (apply) {
        apply.onclick = () => {
            const next = new URLSearchParams();
            const fields = [
                ["state", "f-state"],
                ["type", "f-type"],
                ["priority", "f-priority"],
                ["epic", "f-epic"],
                ["grep", "f-grep"],
            ];
            for (const [key, id] of fields) {
                const v = document.getElementById(id).value.trim();
                if (v) next.set(key, v);
            }
            const scope = document.querySelector("input[name='scope']:checked")?.value;
            if (scope === "closed") next.set("closed", "true");
            else if (scope === "all") next.set("all", "true");
            setHash("issues", next);
        };
    }
    if (clear) {
        clear.onclick = () => setHash("issues", new URLSearchParams());
    }
}

function wireRowClicks() {
    document.querySelectorAll("tr.row").forEach((row) => {
        row.onclick = () => {
            window.location.hash = `#/issues/${row.dataset.id}`;
        };
    });
}

// ---------- Issue detail ----------

async function renderIssue(id) {
    app.innerHTML = `<div class="empty">loading…</div>`;
    try {
        const [issue, comments] = await Promise.all([
            getJSON(`/issues/${id}`),
            getJSON(`/issues/${id}/comments`),
        ]);
        const meta = [
            `<span class="tag">${issue.type}</span>`,
            `<span class="tag">${issue.state}</span>`,
            issue.priority ? `<span class="tag priority-${issue.priority}">${issue.priority}</span>` : "",
            issue.blocker ? `<span class="tag">blocker:${issue.blocker}</span>` : "",
            issue.epic ? `<span class="tag">epic:${escapeHtml(issue.epic)}</span>` : "",
        ]
            .filter((x) => x)
            .join(" ");

        const deps = `
            <h3>Dependencies</h3>
            <p>
                Blocks: ${issue.blocks.length === 0 ? "(none)" : issue.blocks.map((id) => `<a href="#/issues/${id}">#${id}</a>`).join(", ")}<br>
                Blocked by: ${issue.blocked_by.length === 0 ? "(none)" : issue.blocked_by.map((id) => `<a href="#/issues/${id}">#${id}</a>`).join(", ")}
            </p>`;

        const commentsHtml = comments
            .map((c) => {
                const klass = c.kind === "state-change" ? "state-change" : "";
                const summary =
                    c.kind === "state-change"
                        ? `state-change: ${c.from} → ${c.to}`
                        : c.kind === "blocker-set"
                            ? `blocker-set: ${c.blocker}`
                            : c.kind === "blocker-cleared"
                                ? `blocker-cleared: ${c.blocker}`
                                : "comment";
                return `
                <div class="comment ${klass}">
                    <div class="head">#${String(c.seq).padStart(3, "0")} · ${escapeHtml(c.author)} · ${escapeHtml(c.created)} · ${summary}</div>
                    <div>${escapeHtml(c.body)}</div>
                </div>`;
            })
            .join("");

        const isTerminal = issue.state === "done" || issue.state === "dropped";

        app.innerHTML = `
            <article class="detail">
                <h1>#${issue.id}: ${escapeHtml(issue.title)}</h1>
                <div class="meta">${meta}</div>
                <div class="meta">created ${escapeHtml(issue.created)} by ${escapeHtml(issue.created_by)} · updated ${escapeHtml(issue.updated)}</div>
                ${issue.body ? `<div class="body">${escapeHtml(issue.body)}</div>` : ""}
                ${deps}
                ${isTerminal ? `<div class="notice">This issue is in terminal state '${issue.state}'. No mutations allowed.</div>` : renderIssueActions(issue)}
                <h3>Comments (${comments.length})</h3>
                <div id="comments">${commentsHtml || `<div class="empty">no comments yet</div>`}</div>
            </article>`;

        if (!isTerminal) {
            wireIssueActions(id, issue);
        }
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

const TRANSITION_TARGETS = [
    "spec", "architect", "pm", "plan", "test", "implement",
    "review", "doc", "maintainer", "done", "dropped",
];
const BLOCKER_VALUES = ["maintainer-input", "external", "upstream"];
const TYPE_VALUES = ["spec-gap", "feature", "bug", "arch", "doc", "chore"];

function renderIssueActions(issue) {
    const priorityCurrent = issue.priority ?? "";
    const blockerCurrent = issue.blocker ?? "";
    const epicCurrent = issue.epic ?? "";
    return `
        <h3>Actions</h3>
        <details open>
            <summary>Add comment</summary>
            <textarea id="new-comment" rows="3" placeholder="Markdown body…"></textarea>
            <div class="toolbar"><button id="post-comment">Post</button></div>
        </details>
        <details>
            <summary>Transition state</summary>
            <label>To</label>
            <select id="transition-to">
                ${TRANSITION_TARGETS
                    .filter((t) => t !== issue.state)
                    .map((t) => `<option value="${t}">${t}</option>`)
                    .join("")}
            </select>
            <label>Comment (optional)</label>
            <input id="transition-comment" type="text" placeholder="why">
            <label><input id="transition-override" type="checkbox"> --override (maintainer-only; cannot target terminal)</label>
            <div class="toolbar"><button id="do-transition">Transition</button></div>
        </details>
        <details>
            <summary>Close</summary>
            <label>Target</label>
            <select id="close-target">
                <option value="done">done</option>
                <option value="dropped">dropped</option>
            </select>
            <label>Closure comment (required)</label>
            <input id="close-comment" type="text" placeholder="why">
            <div class="toolbar"><button id="do-close" class="danger">Close issue</button></div>
        </details>
        <details>
            <summary>Blocker</summary>
            <label>Set</label>
            <select id="blocker-value">
                ${BLOCKER_VALUES.map((b) => `<option value="${b}" ${b === blockerCurrent ? "selected" : ""}>${b}</option>`).join("")}
            </select>
            <label>Comment (optional)</label>
            <input id="blocker-comment" type="text" placeholder="context">
            <div class="toolbar">
                <button id="do-set-blocker">${issue.blocker ? "Update" : "Set"} blocker</button>
                ${issue.blocker ? `<button id="do-clear-blocker">Clear</button>` : ""}
            </div>
        </details>
        <details>
            <summary>Priority</summary>
            <label>Set</label>
            <select id="priority-value">
                <option value="p0" ${priorityCurrent === "p0" ? "selected" : ""}>p0</option>
                <option value="p1" ${priorityCurrent === "p1" ? "selected" : ""}>p1</option>
                <option value="p2" ${priorityCurrent === "p2" ? "selected" : ""}>p2</option>
            </select>
            <div class="toolbar">
                <button id="do-set-priority">Set</button>
                ${issue.priority ? `<button id="do-clear-priority">Clear</button>` : ""}
            </div>
        </details>
        <details>
            <summary>Edit title / type / epic</summary>
            <label>Title</label>
            <input id="edit-title" type="text" value="${escapeHtml(issue.title)}">
            <label>Type</label>
            <select id="edit-type">
                ${TYPE_VALUES.map((t) => `<option value="${t}" ${t === issue.type ? "selected" : ""}>${t}</option>`).join("")}
            </select>
            <label>Epic (kebab slug)</label>
            <input id="edit-epic" type="text" value="${escapeHtml(epicCurrent)}" placeholder="e.g. cli-foundation">
            <div class="toolbar"><button id="do-edit">Save</button></div>
        </details>
        <details>
            <summary>Dependencies</summary>
            <label>Add edge: this issue blocks</label>
            <input id="dep-add-to" type="number" placeholder="issue id">
            <label>Rationale (optional)</label>
            <input id="dep-add-rationale" type="text" placeholder="why">
            <div class="toolbar"><button id="do-add-dep">Add edge</button></div>
            <label>Or: remove an existing outgoing edge to</label>
            <input id="dep-remove-to" type="number" placeholder="issue id">
            <div class="toolbar"><button id="do-remove-dep">Remove edge</button></div>
        </details>`;
}

function wireIssueActions(id, issue) {
    const reload = () => renderIssue(id);
    const reloadOnSuccess = async (label, fn) => {
        try {
            await fn();
            reload();
        } catch (e) {
            alert(`${label} failed: ${e.message}`);
        }
    };

    document.getElementById("post-comment").onclick = () =>
        reloadOnSuccess("comment", async () => {
            const body = document.getElementById("new-comment").value.trim();
            if (!body) throw new Error("body required");
            await postJSON(`/issues/${id}/comments`, { body });
        });

    document.getElementById("do-transition").onclick = () =>
        reloadOnSuccess("transition", async () => {
            const to = document.getElementById("transition-to").value;
            const comment = document.getElementById("transition-comment").value.trim() || undefined;
            const override = document.getElementById("transition-override").checked;
            const body = { to };
            if (comment) body.comment = comment;
            if (override) body.override = true;
            await postJSON(`/issues/${id}/transitions`, body);
        });

    document.getElementById("do-close").onclick = () =>
        reloadOnSuccess("close", async () => {
            const to = document.getElementById("close-target").value;
            const comment = document.getElementById("close-comment").value.trim();
            if (!comment) throw new Error("closure comment required");
            await postJSON(`/issues/${id}/transitions`, { to, comment });
        });

    document.getElementById("do-set-blocker").onclick = () =>
        reloadOnSuccess("blocker set", async () => {
            const blocker = document.getElementById("blocker-value").value;
            const comment = document.getElementById("blocker-comment").value.trim() || undefined;
            const body = { blocker };
            if (comment) body.comment = comment;
            await putJSON(`/issues/${id}/blocker`, body);
        });

    if (issue.blocker) {
        const btn = document.getElementById("do-clear-blocker");
        if (btn) {
            btn.onclick = () =>
                reloadOnSuccess("blocker clear", async () => {
                    const comment = document.getElementById("blocker-comment").value.trim();
                    const body = comment ? { comment } : {};
                    await deleteJSON(`/issues/${id}/blocker`, body);
                });
        }
    }

    document.getElementById("do-set-priority").onclick = () =>
        reloadOnSuccess("priority set", async () => {
            const priority = document.getElementById("priority-value").value;
            await putJSON(`/issues/${id}/priority`, { priority });
        });

    if (issue.priority) {
        const btn = document.getElementById("do-clear-priority");
        if (btn) {
            btn.onclick = () =>
                reloadOnSuccess("priority clear", async () => {
                    await deleteJSON(`/issues/${id}/priority`);
                });
        }
    }

    document.getElementById("do-edit").onclick = () =>
        reloadOnSuccess("edit", async () => {
            const title = document.getElementById("edit-title").value.trim();
            const type = document.getElementById("edit-type").value;
            const epic = document.getElementById("edit-epic").value.trim();
            const body = {};
            if (title && title !== issue.title) body.title = title;
            if (type !== issue.type) body.type = type;
            if (epic && epic !== (issue.epic ?? "")) body.epic = epic;
            if (Object.keys(body).length === 0) throw new Error("no changes");
            await patchJSON(`/issues/${id}`, body);
        });

    document.getElementById("do-add-dep").onclick = () =>
        reloadOnSuccess("dep add", async () => {
            const to = Number(document.getElementById("dep-add-to").value);
            if (!Number.isFinite(to) || to <= 0) throw new Error("target id required");
            const rationale = document.getElementById("dep-add-rationale").value.trim() || undefined;
            const body = { from: id, to };
            if (rationale) body.rationale = rationale;
            await postJSON(`/dependencies`, body);
        });

    document.getElementById("do-remove-dep").onclick = () =>
        reloadOnSuccess("dep remove", async () => {
            const to = Number(document.getElementById("dep-remove-to").value);
            if (!Number.isFinite(to) || to <= 0) throw new Error("target id required");
            await deleteJSON(`/dependencies/${id}/${to}`);
        });
}

// ---------- Dependencies graph (web-ui.md#R7) ----------

const STATE_COLORS = {
    spec: "#6cb6ff",
    architect: "#aa80ff",
    pm: "#ff80c0",
    plan: "#f0a060",
    test: "#f0d060",
    implement: "#80c080",
    review: "#60c0c0",
    doc: "#c0a0e0",
    maintainer: "#f06060",
    done: "#666",
    dropped: "#444",
};

const PRIORITY_RADIUS = { p0: 22, p1: 17, p2: 13 };
const DEFAULT_RADIUS = 13;

async function renderDeps(params) {
    app.innerHTML = `<div class="empty">loading graph…</div>`;
    try {
        const includeTerminal = params.get("all") === "true";
        const focusId = params.get("focus") ? Number(params.get("focus")) : null;
        const hops = params.get("hops") ? Math.max(1, Number(params.get("hops"))) : 1;
        const collapsedCsv = params.get("collapsed") || "";
        const collapsed = new Set(
            collapsedCsv.split(",").map((s) => s.trim()).filter(Boolean),
        );

        const issues = await getJSON("/issues?all=true");
        let nodes = includeTerminal
            ? issues
            : issues.filter((i) => i.state !== "done" && i.state !== "dropped");
        const nodeById = new Map(nodes.map((n) => [n.id, n]));
        let edges = collectEdges(nodes);

        if (focusId != null && nodeById.has(focusId)) {
            const subset = nHopNeighborhood(focusId, edges, hops);
            nodes = nodes.filter((n) => subset.has(n.id));
            edges = edges.filter((e) => subset.has(e.from) && subset.has(e.to));
        }

        // Apply collapse (issue #9): replace each collapsed-epic's members
        // with a single placeholder node and rewire edges.
        const collapsedResult = collapseClusters(nodes, edges, collapsed);
        const effNodes = collapsedResult.nodes;
        const effEdges = collapsedResult.edges;

        if (effNodes.length === 0) {
            app.innerHTML = `
                <h2>Dependencies</h2>
                ${renderDepsToolbar(includeTerminal, focusId, hops, collapsed)}
                <div class="empty">No issues to graph.</div>`;
            wireDepsToolbar(collapsed);
            return;
        }

        const layers = layerize(effNodes, effEdges);
        const layout = layoutLayers(layers);
        const svg = renderGraphSvg(effNodes, effEdges, layout);

        app.innerHTML = `
            <h2>Dependencies (${effNodes.length} nodes, ${effEdges.length} edges)</h2>
            <p class="meta">Edges flow downward: a node above blocks a node below it. Click a node for details. Click an epic label to collapse its cluster; click a collapsed placeholder to expand it.</p>
            ${renderDepsToolbar(includeTerminal, focusId, hops, collapsed)}
            <div id="graph-container">${svg}</div>
            <aside id="graph-panel" class="notice" style="display: none;"></aside>`;
        wireDepsToolbar(collapsed);
        wireGraphInteraction(nodeById, collapsed);
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

function renderDepsToolbar(includeTerminal, focusId, hops, collapsed) {
    const list = collapsed && collapsed.size > 0
        ? `<div class="meta" style="margin-top: 0.25rem;">collapsed: ${[...collapsed]
            .map((e) => `<span class="tag">${escapeHtml(e)}</span>`)
            .join(" ")}</div>`
        : "";
    return `
        <div class="toolbar">
            <label><input type="checkbox" id="deps-include-terminal" ${includeTerminal ? "checked" : ""}> include terminal (done/dropped)</label>
            <label>focus on issue <input id="deps-focus" type="number" value="${focusId ?? ""}" placeholder="id" style="width: 6em;"></label>
            <label>hops <input id="deps-hops" type="number" value="${hops}" min="1" max="10" style="width: 4em;"></label>
            <button id="deps-apply">Apply</button>
            <button id="deps-clear-focus">Clear focus</button>
            ${collapsed && collapsed.size > 0 ? `<button id="deps-expand-all">Expand all epics</button>` : ""}
        </div>${list}`;
}

function wireDepsToolbar(collapsed) {
    const apply = document.getElementById("deps-apply");
    const clearFocus = document.getElementById("deps-clear-focus");
    const expandAll = document.getElementById("deps-expand-all");
    if (apply) {
        apply.onclick = () => {
            const next = currentDepsParams();
            if (collapsed && collapsed.size > 0) {
                next.set("collapsed", [...collapsed].join(","));
            }
            setHash("deps", next);
        };
    }
    if (clearFocus) {
        clearFocus.onclick = () => {
            const next = new URLSearchParams();
            if (document.getElementById("deps-include-terminal").checked) next.set("all", "true");
            if (collapsed && collapsed.size > 0) {
                next.set("collapsed", [...collapsed].join(","));
            }
            setHash("deps", next);
        };
    }
    if (expandAll) {
        expandAll.onclick = () => {
            const next = currentDepsParams();
            setHash("deps", next);
        };
    }
}

/// Snapshot the toolbar's current focus / hops / scope into a URLSearchParams.
function currentDepsParams() {
    const next = new URLSearchParams();
    if (document.getElementById("deps-include-terminal")?.checked) next.set("all", "true");
    const focus = document.getElementById("deps-focus")?.value.trim();
    if (focus) {
        next.set("focus", focus);
        const hops = document.getElementById("deps-hops")?.value.trim();
        if (hops && hops !== "1") next.set("hops", hops);
    }
    return next;
}

/// Replace each collapsed-epic's members with a single placeholder node;
/// rewire edges. Returns { nodes, edges } with the collapse applied.
function collapseClusters(nodes, edges, collapsedSet) {
    if (!collapsedSet || collapsedSet.size === 0) {
        return { nodes, edges };
    }
    const memberToPlaceholderId = new Map();
    const placeholderByEpic = new Map();
    const survivors = [];
    for (const n of nodes) {
        if (n.epic && collapsedSet.has(n.epic)) {
            if (!placeholderByEpic.has(n.epic)) {
                placeholderByEpic.set(n.epic, {
                    id: `cluster:${n.epic}`,
                    title: n.epic,
                    type: "epic",
                    state: "epic",
                    priority: null,
                    blocker: null,
                    epic: n.epic,
                    blocked_by: [],
                    blocks: [],
                    members: [],
                    isPlaceholder: true,
                });
            }
            const ph = placeholderByEpic.get(n.epic);
            ph.members.push(n.id);
            memberToPlaceholderId.set(n.id, ph.id);
        } else {
            survivors.push(n);
        }
    }
    const newNodes = [...survivors, ...placeholderByEpic.values()];
    const newEdges = [];
    const seen = new Set();
    for (const e of edges) {
        const from = memberToPlaceholderId.get(e.from) ?? e.from;
        const to = memberToPlaceholderId.get(e.to) ?? e.to;
        if (from === to) continue; // intra-cluster edge dropped
        const key = `${from}->${to}`;
        if (seen.has(key)) continue;
        seen.add(key);
        newEdges.push({ from, to });
    }
    return { nodes: newNodes, edges: newEdges };
}

function collectEdges(nodes) {
    const ids = new Set(nodes.map((n) => n.id));
    const edges = [];
    for (const n of nodes) {
        for (const t of n.blocks) {
            if (ids.has(t)) edges.push({ from: n.id, to: t });
        }
    }
    return edges;
}

function nHopNeighborhood(focus, edges, hops) {
    // Both directions: ancestors (things that block focus) and descendants
    // (things focus blocks).
    const inEdges = new Map();
    const outEdges = new Map();
    for (const e of edges) {
        if (!outEdges.has(e.from)) outEdges.set(e.from, []);
        if (!inEdges.has(e.to)) inEdges.set(e.to, []);
        outEdges.get(e.from).push(e.to);
        inEdges.get(e.to).push(e.from);
    }
    const visited = new Set([focus]);
    let frontier = [focus];
    for (let i = 0; i < hops; i++) {
        const next = new Set();
        for (const id of frontier) {
            for (const adj of [...(outEdges.get(id) || []), ...(inEdges.get(id) || [])]) {
                if (!visited.has(adj)) {
                    visited.add(adj);
                    next.add(adj);
                }
            }
        }
        frontier = [...next];
    }
    return visited;
}

function layerize(nodes, edges) {
    // Longest path from any source. Source = node with no incoming edges
    // (no `from` value in `from→to` set).
    const nodeIds = new Set(nodes.map((n) => n.id));
    const inDegree = new Map(nodes.map((n) => [n.id, 0]));
    const outAdj = new Map(nodes.map((n) => [n.id, []]));
    for (const e of edges) {
        if (!nodeIds.has(e.from) || !nodeIds.has(e.to)) continue;
        inDegree.set(e.to, (inDegree.get(e.to) || 0) + 1);
        outAdj.get(e.from).push(e.to);
    }
    const layer = new Map(nodes.map((n) => [n.id, 0]));
    // Kahn-like: process sources first, propagate +1 to successors.
    const queue = [...nodes.filter((n) => inDegree.get(n.id) === 0).map((n) => n.id)];
    const remaining = new Map(inDegree);
    while (queue.length > 0) {
        const id = queue.shift();
        for (const next of outAdj.get(id) || []) {
            layer.set(next, Math.max(layer.get(next), layer.get(id) + 1));
            remaining.set(next, remaining.get(next) - 1);
            if (remaining.get(next) === 0) queue.push(next);
        }
    }
    // Group nodes by layer.
    const byLayer = new Map();
    for (const n of nodes) {
        const l = layer.get(n.id) ?? 0;
        if (!byLayer.has(l)) byLayer.set(l, []);
        byLayer.get(l).push(n);
    }
    for (const arr of byLayer.values()) {
        // Issue #9: within a layer, sort by epic first (no-epic group
        // first, then epic groups alphabetically by slug), then by id
        // within each group. This makes epic-cluster spans contiguous so
        // the SVG layout can draw a single background rect per cluster.
        arr.sort((a, b) => {
            const ae = a.epic || "";
            const be = b.epic || "";
            if (ae !== be) {
                if (ae === "") return -1;
                if (be === "") return 1;
                return ae < be ? -1 : 1;
            }
            const ai = String(a.id);
            const bi = String(b.id);
            return ai < bi ? -1 : ai > bi ? 1 : 0;
        });
    }
    return byLayer;
}

function layoutLayers(byLayer) {
    const layers = [...byLayer.entries()].sort((a, b) => a[0] - b[0]);
    const positions = new Map();
    const clusters = []; // {epic, layer, x_min, x_max, y, count}
    const nodeWidth = 80;
    const nodeHeight = 90;
    const padding = 30;
    const maxNodesInLayer = Math.max(1, ...layers.map(([, ns]) => ns.length));
    const width = Math.max(600, maxNodesInLayer * nodeWidth + padding * 2);
    layers.forEach(([layerIdx, layerNodes], i) => {
        const y = padding + i * nodeHeight + 30;
        const totalW = layerNodes.length * nodeWidth;
        const xStart = (width - totalW) / 2 + nodeWidth / 2;
        layerNodes.forEach((n, j) => {
            positions.set(n.id, { x: xStart + j * nodeWidth, y, layer: layerIdx });
        });
        // Detect contiguous epic runs and record cluster spans for the
        // rendering pass.
        let runStart = 0;
        for (let j = 1; j <= layerNodes.length; j++) {
            const prevEpic = layerNodes[j - 1].epic || "";
            const curEpic = j < layerNodes.length ? (layerNodes[j].epic || "") : null;
            const ended = j === layerNodes.length || curEpic !== prevEpic;
            if (ended && prevEpic) {
                clusters.push({
                    epic: prevEpic,
                    layer: layerIdx,
                    x_min: xStart + runStart * nodeWidth,
                    x_max: xStart + (j - 1) * nodeWidth,
                    y,
                    count: j - runStart,
                });
            }
            if (curEpic !== prevEpic) runStart = j;
        }
    });
    const height = layers.length * nodeHeight + padding * 2;
    return { positions, width, height, clusters };
}

function renderGraphSvg(nodes, edges, layout) {
    const { positions, width, height, clusters } = layout;
    const arrowDef = `
        <defs>
            <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5"
                    markerUnits="strokeWidth" markerWidth="8" markerHeight="8" orient="auto">
                <path d="M 0 0 L 10 5 L 0 10 z" fill="var(--muted)"/>
            </marker>
        </defs>`;
    // Issue #9: cluster background + label render BEFORE edges and nodes
    // so they sit at the back of the z-order. Translucent fill + label
    // above the rect; label is clickable in wireGraphInteraction.
    const clusterSvg = (clusters || []).map((c) => {
        const padX = 18;
        const padY = 26;
        const rectX = c.x_min - padX;
        const rectY = c.y - padY;
        const rectW = (c.x_max - c.x_min) + padX * 2;
        const rectH = padY * 2 + 28;
        const hue = epicHue(c.epic);
        const fill = `hsla(${hue}, 60%, 50%, 0.10)`;
        const stroke = `hsla(${hue}, 60%, 50%, 0.35)`;
        return `
            <g class="epic-cluster" data-epic="${escapeHtml(c.epic)}">
                <rect class="epic-cluster-bg" x="${rectX}" y="${rectY}" width="${rectW}" height="${rectH}"
                      rx="8" ry="8" fill="${fill}" stroke="${stroke}" stroke-width="1"/>
                <text class="epic-label" x="${(c.x_min + c.x_max) / 2}" y="${rectY - 4}"
                      text-anchor="middle" font-size="11" font-weight="600"
                      fill="hsl(${hue}, 70%, 70%)">${escapeHtml(c.epic)}</text>
            </g>`;
    }).join("");
    const edgeSvg = edges
        .map((e) => {
            const a = positions.get(e.from);
            const b = positions.get(e.to);
            if (!a || !b) return "";
            // Offset so the line ends at the node's edge, not center.
            return `<line x1="${a.x}" y1="${a.y + 22}" x2="${b.x}" y2="${b.y - 22}"
                    stroke="var(--muted)" stroke-width="1.5" marker-end="url(#arrow)"/>`;
        })
        .join("");
    const nodeSvg = nodes
        .map((n) => {
            const p = positions.get(n.id);
            if (!p) return "";
            // Collapsed-cluster placeholder: rounded rect with "epic (N)" label.
            if (n.isPlaceholder) {
                const w = 110;
                const h = 38;
                const x = p.x - w / 2;
                const y = p.y - h / 2;
                const hue = epicHue(n.epic);
                const fill = `hsl(${hue}, 50%, 30%)`;
                return `
                    <g class="graph-node graph-placeholder" data-cluster-epic="${escapeHtml(n.epic)}" style="cursor: pointer;">
                        <rect x="${x}" y="${y}" width="${w}" height="${h}" rx="6" ry="6"
                              fill="${fill}" stroke="var(--fg)" stroke-width="2"/>
                        <text x="${p.x}" y="${p.y + 4}" text-anchor="middle" font-size="11"
                              fill="var(--fg)" font-weight="600">${escapeHtml(n.epic)} (${n.members.length})</text>
                    </g>`;
            }
            const r = PRIORITY_RADIUS[n.priority] ?? DEFAULT_RADIUS;
            const fill = STATE_COLORS[n.state] ?? "#888";
            const strokeW = n.priority ? 3 : 1;
            const blocker = n.blocker
                ? `<circle cx="${p.x + r - 3}" cy="${p.y - r + 3}" r="5" fill="var(--warn)" stroke="var(--bg)" stroke-width="1.5"/>`
                : "";
            return `
                <g class="graph-node" data-id="${n.id}" style="cursor: pointer;">
                    <circle cx="${p.x}" cy="${p.y}" r="${r}" fill="${fill}" stroke="var(--fg)" stroke-width="${strokeW}"/>
                    <text x="${p.x}" y="${p.y + 4}" text-anchor="middle" font-size="11" fill="var(--bg)" font-weight="600">${n.id}</text>
                    ${blocker}
                    <text x="${p.x}" y="${p.y + r + 14}" text-anchor="middle" font-size="10" fill="var(--fg)">${escapeHtml(truncate(n.title, 14))}</text>
                </g>`;
        })
        .join("");
    return `
        <svg width="${width}" height="${height}" xmlns="http://www.w3.org/2000/svg" style="background: var(--bg-alt); border-radius: 4px;">
            ${arrowDef}
            ${clusterSvg}
            ${edgeSvg}
            ${nodeSvg}
        </svg>
        <div class="meta" style="margin-top: 0.5rem;">
            ${Object.entries(STATE_COLORS)
                .map(([s, c]) => `<span class="tag" style="background: ${c}; color: var(--bg); border-color: ${c};">${s}</span>`)
                .join(" ")}
            · larger circle = higher priority · orange dot = blocker set · click epic label to collapse cluster
        </div>`;
}

/// Deterministic string-to-hue so each epic gets a stable visual color.
function epicHue(epic) {
    let h = 0;
    for (let i = 0; i < epic.length; i++) {
        h = (h * 31 + epic.charCodeAt(i)) | 0;
    }
    return Math.abs(h) % 360;
}

function truncate(s, max) {
    if (!s) return "";
    if (s.length <= max) return s;
    return s.slice(0, max - 1) + "…";
}

function wireGraphInteraction(nodeById, collapsed) {
    const currentCollapsed = new Set(collapsed || []);

    document.querySelectorAll("g.graph-node").forEach((g) => {
        g.addEventListener("click", (ev) => {
            // Placeholder → expand the cluster (issue #9).
            if (g.classList.contains("graph-placeholder")) {
                ev.stopPropagation();
                const epic = g.dataset.clusterEpic;
                const next = currentCollapsed;
                next.delete(epic);
                const params = currentDepsParams();
                if (next.size > 0) params.set("collapsed", [...next].join(","));
                else params.delete("collapsed");
                setHash("deps", params);
                return;
            }
            const id = Number(g.dataset.id);
            const n = nodeById.get(id);
            if (!n) return;
            const panel = document.getElementById("graph-panel");
            panel.style.display = "block";
            panel.innerHTML = `
                <strong>#${n.id}: ${escapeHtml(n.title)}</strong><br>
                <span class="tag">${n.type}</span>
                <span class="tag">${n.state}</span>
                ${n.priority ? `<span class="tag priority-${n.priority}">${n.priority}</span>` : ""}
                ${n.blocker ? `<span class="tag">blocker:${n.blocker}</span>` : ""}
                ${n.epic ? `<span class="tag">epic:${escapeHtml(n.epic)}</span>` : ""}
                <br>
                Blocks: ${n.blocks.length === 0 ? "(none)" : n.blocks.map((id) => `<a href="#/issues/${id}">#${id}</a>`).join(", ")}<br>
                Blocked by: ${n.blocked_by.length === 0 ? "(none)" : n.blocked_by.map((id) => `<a href="#/issues/${id}">#${id}</a>`).join(", ")}<br>
                <a href="#/issues/${n.id}">open detail →</a>`;
        });
    });

    // Issue #9: epic label click → collapse the cluster.
    document.querySelectorAll("g.epic-cluster text.epic-label").forEach((label) => {
        label.style.cursor = "pointer";
        label.addEventListener("click", (ev) => {
            ev.stopPropagation();
            const epic = label.parentElement.dataset.epic;
            const next = currentCollapsed;
            next.add(epic);
            const params = currentDepsParams();
            params.set("collapsed", [...next].join(","));
            setHash("deps", params);
        });
    });
}

// ---------- Schedule screen (web-ui.md#R8) ----------

let scheduleActionableOnly = false;

async function renderSchedule() {
    app.innerHTML = `<div class="empty">computing schedule…</div>`;
    try {
        const qs = scheduleActionableOnly ? "?actionable=true" : "";
        const rows = await getJSON(`/scheduler/queue${qs}`);
        if (rows.length === 0) {
            app.innerHTML = `<h2>Schedule</h2>
                <div class="toolbar">
                    <label><input type="checkbox" id="actionable-toggle" ${scheduleActionableOnly ? "checked" : ""}> actionable only</label>
                </div>
                <div class="empty">No active issues.</div>`;
            wireToggle();
            return;
        }
        app.innerHTML = `
            <h2>Schedule (${rows.length})</h2>
            <p class="meta">Ranked by effective priority. Higher = work on this sooner. <code>score = base + α · Σ score(blocked)</code> per dep-graph.md#R3.2.</p>
            <div class="toolbar">
                <label><input type="checkbox" id="actionable-toggle" ${scheduleActionableOnly ? "checked" : ""}> actionable only</label>
            </div>
            <table>
                <thead><tr>
                    <th>RANK</th><th>ID</th><th>TITLE</th><th>STATE</th><th>TYPE</th>
                    <th>PRI</th><th>SCORE</th><th>OVR</th><th>EFF</th><th></th>
                </tr></thead>
                <tbody>${rows
                    .map((r, i) => {
                        const row = `
                    <tr class="row" data-id="${r.id}">
                        <td>${i + 1}</td>
                        <td>#${r.id}</td>
                        <td>${escapeHtml(r.title)}${r.actionable ? "" : ` <span class="tag">blocked</span>`}${r.in_cycle ? ` <span class="tag">cycle</span>` : ""}</td>
                        <td><span class="tag">${r.state}</span></td>
                        <td><span class="tag">${r.type}</span></td>
                        <td>${r.priority ? `<span class="tag priority-${r.priority}">${r.priority}</span>` : "-"}</td>
                        <td>${r.score.toFixed(2)}</td>
                        <td>${r.override == null ? "-" : `<span class="tag">${r.override}</span>`}</td>
                        <td>${r.effective_priority.toFixed(2)}</td>
                        <td>
                            <button data-act="set-override" data-id="${r.id}" data-current="${r.override == null ? "" : r.override}">override</button>
                            ${r.override == null ? "" : `<button data-act="clear-override" data-id="${r.id}">clear</button>`}
                        </td>
                    </tr>`;
                        return row;
                    })
                    .join("")}</tbody>
            </table>`;
        wireToggle();
        wireRowClicks();
        wireOverrideButtons();
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

function wireToggle() {
    const toggle = document.getElementById("actionable-toggle");
    if (toggle) {
        toggle.onchange = () => {
            scheduleActionableOnly = toggle.checked;
            renderSchedule();
        };
    }
}

function wireOverrideButtons() {
    document.querySelectorAll("[data-act='set-override']").forEach((btn) => {
        btn.onclick = async (ev) => {
            ev.stopPropagation();
            const id = Number(btn.dataset.id);
            const current = btn.dataset.current;
            const v = prompt(
                `Set effective-priority override for issue #${id}.\nHigher value ranks higher. Leave blank to cancel.`,
                current,
            );
            if (v == null || v.trim() === "") return;
            const num = Number(v);
            if (!Number.isFinite(num)) {
                alert("Override must be a finite number.");
                return;
            }
            try {
                await postJSON("/scheduler/override", { issue: id, value: num });
                renderSchedule();
            } catch (e) {
                alert("override failed: " + e.message);
            }
        };
    });
    document.querySelectorAll("[data-act='clear-override']").forEach((btn) => {
        btn.onclick = async (ev) => {
            ev.stopPropagation();
            const id = Number(btn.dataset.id);
            try {
                await postJSON("/scheduler/override", { issue: id, clear: true });
                renderSchedule();
            } catch (e) {
                alert("clear failed: " + e.message);
            }
        };
    });
}

// ---------- Daemon screen ----------

async function renderDaemon() {
    app.innerHTML = `<div class="empty">loading…</div>`;
    try {
        const status = await getJSON("/daemon");
        app.innerHTML = `
            <h2>Daemon</h2>
            <p>
                state: <span class="tag">${status.state}</span><br>
                pid: ${status.pid}<br>
                port: ${status.port}<br>
                uptime: ${status.uptime_seconds}s<br>
                version: ${status.version}
            </p>
            <div class="toolbar">
                <button id="reindex">Reindex</button>
                <button id="shutdown" class="danger">Shutdown</button>
            </div>
            <div id="op-result"></div>`;
        document.getElementById("reindex").onclick = async () => {
            const r = document.getElementById("op-result");
            r.textContent = "reindexing…";
            try {
                const stats = await postJSON("/daemon/reindex", {});
                r.innerHTML = `<div class="notice">Reindexed: ${stats.issues} issues, ${stats.comments} comments, ${stats.edges} edges.</div>`;
            } catch (e) {
                r.innerHTML = `<div class="notice">reindex failed: ${escapeHtml(e.message)}</div>`;
            }
        };
        document.getElementById("shutdown").onclick = async () => {
            if (!confirm("Stop the daemon? The web UI will disconnect.")) return;
            try {
                await postJSON("/daemon/shutdown", {});
                document.getElementById("op-result").innerHTML =
                    `<div class="notice">Shutdown requested. Connection will drop shortly.</div>`;
            } catch (e) {
                alert("shutdown failed: " + e.message);
            }
        };
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

// ---------- Config screen (web-ui.md#R10) ----------

const RESTART_REQUIRED_KEYS = new Set(["daemon.port", "daemon.bind"]);

async function renderConfig() {
    app.innerHTML = `<div class="empty">loading…</div>`;
    try {
        const cfg = await getJSON("/config");
        app.innerHTML = `
            <h2>Configuration</h2>
            <p class="meta">Edits PATCH only the changed fields. Keys flagged "restart" require <code>dwarven daemon restart</code> to take effect.</p>
            <form id="config-form">
                ${renderConfigSection("repo", cfg.repo, { id: { readonly: true } })}
                ${renderConfigSection("counters", cfg.counters, { next_issue_id: { warn: "internal counter; edit at your own risk" } })}
                ${renderConfigSection("daemon", cfg.daemon, { port: { restart: true }, bind: { restart: true } })}
                ${renderConfigSection("scheduler", flattenForForm(cfg.scheduler), {})}
                ${renderConfigSection("triage", cfg.triage, {})}
                <div class="toolbar">
                    <button type="button" id="config-save">Save changes</button>
                    <button type="button" id="config-reset">Reset</button>
                </div>
                <div id="config-result"></div>
            </form>
            <h3>Raw config.toml</h3>
            <pre class="body" id="config-raw">${escapeHtml(JSON.stringify(cfg, null, 2))}</pre>`;
        wireConfigSave(cfg);
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
}

function renderConfigSection(name, table, hints) {
    if (!table || typeof table !== "object") return "";
    const rows = Object.entries(table)
        .map(([key, value]) => {
            const fullKey = `${name}.${key}`;
            const hint = hints[key] || {};
            const restart = RESTART_REQUIRED_KEYS.has(fullKey) || hint.restart;
            const readonly = hint.readonly ? "readonly" : "";
            const flag = restart ? `<span class="tag">restart</span>` : "";
            const warn = hint.warn ? `<div class="meta">${hint.warn}</div>` : "";
            const display =
                typeof value === "object" && value !== null
                    ? JSON.stringify(value)
                    : String(value);
            const inputType =
                typeof value === "number" ? "number" : typeof value === "boolean" ? "checkbox" : "text";
            const inputAttr =
                inputType === "checkbox"
                    ? `type="checkbox" ${value ? "checked" : ""}`
                    : `type="${inputType}" value="${escapeHtml(display)}"`;
            return `
                <div style="margin: 0.5rem 0;">
                    <label>${fullKey} ${flag}</label>
                    <input data-key="${fullKey}" data-orig="${escapeHtml(display)}" data-kind="${inputType}" ${inputAttr} ${readonly}>
                    ${warn}
                </div>`;
        })
        .join("");
    return `<details ${["daemon", "scheduler"].includes(name) ? "open" : ""}>
        <summary>[${name}]</summary>
        ${rows}
    </details>`;
}

function flattenForForm(obj, prefix = "") {
    // The scheduler.priority_weights subtable shows as nested keys in the
    // form (e.g., "priority_weights.p0").
    if (!obj || typeof obj !== "object") return obj;
    const out = {};
    for (const [k, v] of Object.entries(obj)) {
        if (v && typeof v === "object" && !Array.isArray(v)) {
            for (const [k2, v2] of Object.entries(v)) {
                out[`${k}.${k2}`] = v2;
            }
        } else {
            out[k] = v;
        }
    }
    return out;
}

function wireConfigSave(originalCfg) {
    document.getElementById("config-save").onclick = async () => {
        const inputs = document.querySelectorAll("input[data-key]");
        const changes = {};
        let anyChange = false;
        for (const inp of inputs) {
            if (inp.readOnly) continue;
            const key = inp.dataset.key;
            const orig = inp.dataset.orig;
            const kind = inp.dataset.kind;
            const current = kind === "checkbox" ? String(inp.checked) : inp.value;
            if (current === orig) continue;
            anyChange = true;
            const path = key.split(".");
            const value =
                kind === "checkbox"
                    ? inp.checked
                    : kind === "number"
                        ? Number(inp.value)
                        : inp.value;
            insertNested(changes, path, value);
        }
        if (!anyChange) {
            document.getElementById("config-result").innerHTML =
                `<div class="notice">No changes.</div>`;
            return;
        }
        try {
            const resp = await patchJSON("/config", changes);
            const note = resp.requires_restart
                ? `<div class="notice">Saved. <strong>Restart required</strong> for daemon.port / daemon.bind to take effect.</div>`
                : `<div class="notice">Saved.</div>`;
            document.getElementById("config-result").innerHTML = note;
            // Re-render to pick up server-canonical state.
            setTimeout(renderConfig, 800);
        } catch (e) {
            document.getElementById("config-result").innerHTML =
                `<div class="notice">save failed: ${escapeHtml(e.message)}</div>`;
        }
    };
    document.getElementById("config-reset").onclick = () => renderConfig();
    void originalCfg;
}

function insertNested(target, pathParts, value) {
    let cur = target;
    for (let i = 0; i < pathParts.length - 1; i++) {
        const k = pathParts[i];
        if (!cur[k] || typeof cur[k] !== "object") cur[k] = {};
        cur = cur[k];
    }
    cur[pathParts[pathParts.length - 1]] = value;
}

// ---------- SSE subscription ----------

function connectSSE() {
    const es = new EventSource(`${API}/events`);
    setConn("connecting");
    es.onopen = () => setConn("connected");
    es.onerror = () => setConn("disconnected");
    const refresh = () => {
        const r = parseHash();
        if (r.route === "inbox") renderInbox();
        else if (r.route === "issues") renderIssues(r.params);
        else if (r.route === "issue") renderIssue(r.id);
        else if (r.route === "deps") renderDeps(r.params);
        else if (r.route === "schedule") renderSchedule();
        else if (r.route === "daemon") renderDaemon();
        // config screen does not auto-refresh on events
    };
    ["issue.created", "issue.changed", "issue.closed", "comment.added",
     "dependency.added", "dependency.removed", "daemon.reindexed"].forEach((name) => {
        es.addEventListener(name, refresh);
    });
}

connectSSE();
render();
