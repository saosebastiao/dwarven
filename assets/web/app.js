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
