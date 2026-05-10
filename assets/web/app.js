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
    const resp = await fetch(`${API}${path}`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
    });
    if (!resp.ok) {
        const err = await resp.json().catch(() => ({}));
        throw new Error(err.message || `${resp.status} ${resp.statusText}`);
    }
    return resp.json();
}

// ---------- Routing ----------

function parseHash() {
    const h = window.location.hash.replace(/^#\/?/, "");
    if (!h) return { route: "inbox" };
    const parts = h.split("/");
    if (parts[0] === "issues" && parts[1]) return { route: "issue", id: parts[1] };
    return { route: parts[0] };
}

async function render() {
    const r = parseHash();
    document
        .querySelectorAll("nav a")
        .forEach((a) => a.classList.toggle("active", a.dataset.route === r.route || (a.dataset.route === "issues" && r.route === "issue")));
    if (r.route === "inbox") return renderInbox();
    if (r.route === "issues") return renderIssues();
    if (r.route === "issue") return renderIssue(r.id);
    if (r.route === "daemon") return renderDaemon();
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

async function renderIssues() {
    app.innerHTML = `<div class="empty">loading…</div>`;
    try {
        const params = new URLSearchParams(window.location.search);
        const qs = params.toString() ? `?${params.toString()}` : "";
        const items = await getJSON(`/issues${qs}`);
        state.issues = items;
        const filterBar = `
            <div class="toolbar">
                <button data-filter="">all open</button>
                <button data-filter="?closed=true">closed</button>
                <button data-filter="?all=true">all</button>
            </div>`;
        if (items.length === 0) {
            app.innerHTML = `<h2>Issues</h2>${filterBar}<div class="empty">No issues match.</div>`;
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
        document.querySelectorAll("[data-filter]").forEach((btn) => {
            btn.onclick = () => {
                const f = btn.dataset.filter;
                window.location.hash = "#/issues";
                window.location.search = f;
            };
        });
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
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

        app.innerHTML = `
            <article class="detail">
                <h1>#${issue.id}: ${escapeHtml(issue.title)}</h1>
                <div class="meta">${meta}</div>
                <div class="meta">created ${escapeHtml(issue.created)} by ${escapeHtml(issue.created_by)} · updated ${escapeHtml(issue.updated)}</div>
                ${issue.body ? `<div class="body">${escapeHtml(issue.body)}</div>` : ""}
                ${deps}
                <div id="actions">
                    <h3>Add comment</h3>
                    <textarea id="new-comment" rows="3" placeholder="Markdown body…"></textarea>
                    <div class="toolbar">
                        <button id="post-comment">Post</button>
                    </div>
                </div>
                <h3>Comments (${comments.length})</h3>
                ${commentsHtml || `<div class="empty">no comments yet</div>`}
            </article>`;

        document.getElementById("post-comment").onclick = async () => {
            const body = document.getElementById("new-comment").value.trim();
            if (!body) return;
            try {
                await postJSON(`/issues/${id}/comments`, { body });
                renderIssue(id);
            } catch (e) {
                alert("comment failed: " + e.message);
            }
        };
    } catch (e) {
        app.innerHTML = `<div class="empty">load failed: ${escapeHtml(e.message)}</div>`;
    }
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

// ---------- SSE subscription ----------

function connectSSE() {
    const es = new EventSource(`${API}/events`);
    setConn("connecting");
    es.onopen = () => setConn("connected");
    es.onerror = () => setConn("disconnected");
    const refresh = () => {
        const r = parseHash();
        if (r.route === "inbox") renderInbox();
        else if (r.route === "issues") renderIssues();
        else if (r.route === "issue") renderIssue(r.id);
        else if (r.route === "daemon") renderDaemon();
    };
    ["issue.created", "issue.changed", "issue.closed", "comment.added",
     "dependency.added", "dependency.removed", "daemon.reindexed"].forEach((name) => {
        es.addEventListener(name, refresh);
    });
}

connectSSE();
render();
