# githut — CLAUDE.md

## What is githut?

githut is a terminal TUI for discovering, browsing, and acquiring GitHub repositories.
It is NOT a local git manager (that's lazygit). It talks to the GitHub API.

Primary use case: you're on a headless server, SSH session, or just don't want to open
a browser. You want to find a repo, read its README, clone it or fork it — all from
the terminal, keyboard-driven.

## Stack

- Language: Rust (stable)
- TUI: ratatui + crossterm
- Async: tokio
- GitHub API: octocrab
- Git operations: git2 (regular clone) + git CLI shell-out (sparse clone)
- HTTP: reqwest (raw file download fallback for files >1MB)
- Markdown rendering: pulldown-cmark (custom ratatui renderer in src/markdown.rs)
- Error handling: anyhow
- Serialization: serde + serde_json
- URL encoding: urlencoding
- Base64 decoding: base64

## Project Structure

```
githut/
  src/
    main.rs          -- entrypoint, boots tokio runtime, initializes terminal, runs app
    app.rs           -- App struct, central state machine
    types.rs         -- shared types (Repo, SearchResult, AppState, etc.)
    git.rs           -- local git operations (clone, sparse-clone)
    api/
      mod.rs         -- re-exports GithubClient
      auth.rs        -- shells out to `gh auth token` to get token
      client.rs      -- GitHub API calls (search, readme, contents, file) via octocrab
    tui/
      mod.rs
      ui.rs          -- all ratatui rendering logic
      events.rs      -- keyboard input handling, event loop
    markdown.rs      -- custom markdown → Vec<Line> renderer (pulldown-cmark)
    input.rs         -- TextInput struct: cursor movement, arrow keys, path expansion
  Cargo.toml
  flake.nix
  CLAUDE.md
```

## Layout

```
+----------------------------------+-----------------------------+
| [ search: ______________ ]        |                             |
+----------------------------------+  README preview              |
| owner/repo-name      Rust  4.2k  |  (rendered markdown)        |
| short description here...        |                             |
| > owner/selected-repo  Go  891   |                             |
|   description of selected...     |                             |
+----------------------------------+-----------------------------+
 /:search  j/k:nav  J/K:scroll  l:files  c:clone  o:browser  q:quit
```

Left pane: search bar + scrollable results list (50% width)
Right pane: README auto-loads on selection (50% width), debounced 300ms
Bottom bar: keybindings hint

File browser mode (activated with `l`):
```
+----------------------------------+-----------------------------+
| Files — owner/repo/src/          |  file content / preview     |
+----------------------------------+                             |
| ▶ api/                           |                             |
| ▶ tui/                           |                             |
|   main.rs                        |                             |
| > app.rs                         |                             |
+----------------------------------+-----------------------------+
 j/k:nav  J/K:scroll preview  l:open  h:up/back  Esc:back  q:quit
```

## Keybindings

Controls are consistent across all modes.

| Key         | Action                                        |
|-------------|-----------------------------------------------|
| /           | Focus search input                            |
| Enter       | Confirm search                                |
| j / k       | Navigate list (repos or files)                |
| J / K       | Scroll preview pane (readme or file content)  |
| l / Enter   | Open file browser / enter dir / preview file  |
| h           | Go up one dir; at root, back to repo list     |
| Esc         | Back / close overlay                          |
| c           | Clone selected repo / save file (in FileBrowsing) |
| C           | Sparse-clone (prompts path + dirs)            |
| f           | Fork selected repo                            |
| s           | Star / unstar selected repo                   |
| o           | Open selected repo in browser                 |
| r           | Refresh / re-fetch results                    |
| ?           | Toggle help overlay                           |
| q           | Quit                                          |

## Auth

Auth is handled by shelling out to `gh auth token`.
This avoids any OAuth setup — user just needs `gh` installed and authed.

```rust
// auth.rs pattern
let token = std::process::Command::new("gh")
    .args(["auth", "token"])
    .output()?;
let token = String::from_utf8(token.stdout)?.trim().to_string();
// pass to octocrab builder
```

If `gh` is not installed or not authed, githut should show a clear error and exit.

## GitHub API Usage

All calls go through octocrab. Key endpoints:

- Search repos: `GET /search/repositories?q=...`
- Get README: `GET /repos/{owner}/{repo}/readme`
- Get contents (dir listing): `GET /repos/{owner}/{repo}/contents/{path}`
- Get file content: `GET /repos/{owner}/{repo}/contents/{path}` (single file)
- Fork repo: `POST /repos/{owner}/{repo}/forks`
- Star repo: `PUT /user/starred/{owner}/{repo}`
- Unstar repo: `DELETE /user/starred/{owner}/{repo}`
- Check if starred: `GET /user/starred/{owner}/{repo}` (204 = starred, 404 = not)

Rate limits: unauthenticated = 10 req/min search, authenticated = 30 req/min search.
Always use auth. Show rate limit remaining in status bar if possible.

## Git Operations

Regular clone uses git2:
```rust
git2::Repository::clone(url, path)?;
```

Sparse clone shells out to git CLI — git2's `checkout_tree` ignores sparse-checkout
patterns, so this is the only reliable approach:
```
git clone --no-checkout --filter=blob:none --sparse <url> <path>
git -C <path> sparse-checkout set <dirs...>
git -C <path> checkout
```
All git subprocess stdout/stderr is suppressed (Stdio::null) to prevent bleeding into TUI.

Both operations run in `tokio::task::spawn_blocking` and send results back via an
`mpsc::UnboundedChannel` stored on `App`. The main loop drains it each tick.

Path inputs expand `~`, `./`, `../` at submit time via `TextInput::to_path()` in input.rs.

## App State Machine

```
enum AppState {
    Searching,        // user is typing in search bar
    Browsing,         // navigating results list
    FileBrowsing,     // browsing repo file tree
    Cloning,          // clone path input prompt
    SparseCloning,    // sparse-clone path + dirs prompt
    FileSaving,       // save single file path prompt (from FileBrowsing)
    Previewing,       // reserved (unused currently)
    Error(String),    // showing error overlay
    Help,             // showing help overlay
}
```

State transitions are handled in events.rs based on keypress + current state.

README auto-loads with a 300ms debounce — `j/k` updates selection instantly,
fetch fires after 300ms idle. `readme_pending: Option<Instant>` in App tracks this.
Checked in the `run_app` loop in main.rs, not in the event handler.

## Phases

### Phase 1 — Core (DONE)
Goal: working search + browse + clone. Shippable v0.1.

- [x] auth.rs: `gh auth token` integration, error if missing
- [x] github.rs: search repos, return Vec<Repo>
- [x] types.rs: Repo struct, AppState enum, SearchResult
- [x] app.rs: App struct with state, results, selected index, search query
- [x] ui.rs: split layout, results list, search bar, keybindings bar
- [x] events.rs: keyboard loop, j/k navigation, / for search
- [x] github.rs: fetch README for selected repo
- [x] ui.rs: README preview pane with pulldown-cmark rendering
- [x] git.rs: basic clone on `c` keypress with path prompt

### Phase 2 — GitHub Actions (DONE)
Goal: fork, star, open browser.

- [x] github.rs: fork endpoint
- [x] github.rs: star / unstar + check if starred
- [x] ui.rs: star indicator on repo in list (★ when starred)
- [x] events.rs: f, s keybindings
- [x] open crate: browser open on `o`

### Phase 3 — Power Features (DONE)
Goal: sparse clone, language filter, better UX.

- [x] git.rs: sparse-clone via git CLI (--filter=blob:none --sparse)
- [x] events.rs: C for sparse-clone, two-step prompt (path → dirs)
- [x] events.rs: c in FileBrowsing saves single file to local path
- [x] clone/sparse-clone run in background — TUI stays responsive
- [x] TextInput: arrow keys, cursor, Home/End, Delete, path expansion (~, ./, ../)
- [x] ui.rs: language filter tab cycling (Tab key, shown in search bar title)
- [x] github.rs: pass language filter to search query
- [x] ui.rs: rate limit display in status bar (search + core, right-aligned)
- [x] config: ~/.config/githut/config.toml with default_clone_path

### Phase 4 — Your Repos (DONE)
Goal: manage your own repos, not just discover.

- [x] github.rs: list authenticated user's repos (GET /user/repos)
- [x] ui.rs: tab switching — 1=Search, 2=My Repos — tab bar at top
- [x] github.rs: delete (D), rename (R), archive/unarchive (A) via API
- [x] My Repos list shows archived/fork badges, star indicator
- [x] File browser, clone, star, open browser all work from My Repos tab
- [x] Profile README auto-loads when switching to My Repos (if <login>/<login> repo exists)
- [x] File browser uses active_repo() — works correctly from both tabs
- [x] h/Esc in file browser returns to correct tab (MyRepos or Browsing)
- [x] File preview fallback to download_url for files >1MB

### Phase 5 — Profiles (DONE)
Goal: browse any user's public repos and profile info.

- [x] github.rs: get user profile (login, name, bio, followers, following, public repos)
- [x] github.rs: list repos by username
- [x] events.rs: u key on any repo → opens owner's profile view
- [x] ui.rs: profile header (login, name, bio, stats) + their repos list
- [x] profile repos: j/k nav, l file browser, c clone, o open profile, u follow owner
- [x] Esc/h returns to previous state (Browsing, MyRepos, or profile chain)
- [x] file browser correctly tracks origin state via prev_state

### Phase 6 — Issues, PRs & Notifications (DONE)
Goal: browse issues/PRs for any repo, create issues, view notifications.

- [x] client.rs: list_issues (issues + PRs via pull_request field filter), get_issue_comments, create_issue, close_issue
- [x] client.rs: list_notifications, mark_notification_read, mark_all_notifications_read
- [x] types.rs: Issue, IssueComment, Notification, IssueFilter, IssueTab structs/enums
- [x] types.rs: ViewingIssues, ViewingIssue, CreatingIssue, ViewingNotifications AppState variants
- [x] events.rs: i key → opens issues list for selected repo (works from Search, MyRepos, Profile)
- [x] events.rs: Tab to switch Issues/PRs, f to cycle open/closed/all filter, n to create, x to close
- [x] events.rs: l/Enter to open issue detail + comments, Esc/h back to list
- [x] events.rs: 3 key → notifications from anywhere (r mark read, R mark all, f toggle unread filter, o browser)
- [x] ui.rs: issues list (left pane), issue preview (right pane), issue detail with comments (full right pane)
- [x] ui.rs: notifications full-screen list, create issue overlay (title + body fields, Tab to switch)
- [x] ui.rs: tab bar shows [3:Notifications] when active

## Error Handling

Use anyhow for all errors. Surface them to the user via AppState::Error(msg).
Never panic in production code. All GitHub API calls are fallible — handle 403
(rate limit), 404 (not found), 401 (bad token) explicitly with readable messages.

## NixOS / Dev Environment

Use `nix develop` to enter the dev shell (flake.nix).
Do NOT suggest `cargo install` for global tools — add to flake.nix buildInputs.
`cargo-watch` is available in the shell: `cargo watch -x run` for live reloading.

LIBGIT2_SYS_USE_PKG_CONFIG=1 and OPENSSL_NO_VENDOR=1 are set in the shell.
These are required for git2 and octocrab to link correctly on NixOS.

## Distribution & Installation

### PATH handling
Package managers handle PATH automatically. Each install method drops the binary
in a standard location that's already in the user's PATH:
- cargo install → `~/.cargo/bin`
- apt/dnf/pacman → `/usr/bin`
- brew → `/usr/local/bin` or `/opt/homebrew/bin`
- nix → `/etc/profiles/per-user/$USER/bin` or similar

Users never need to manually set PATH if installing through a package manager.

### Recommended rollout order

1. **crates.io** — `cargo publish` once v0.1 is solid
   - Users: `cargo install githut`
   - Zero friction, works on Linux/Mac/Windows
   - Requires Rust installed (fine for target audience)

2. **GitHub Releases + binaries** — set up GitHub Actions CI to cross-compile
   - Targets: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`
   - Attach to release tags automatically
   - Users download binary, put it in PATH manually or via install script
   - Use `cargo-cross` or `cross` for cross-compilation in CI

3. **AUR** — write a `PKGBUILD` (source or bin variant)
   - `githut` = builds from source via cargo
   - `githut-bin` = downloads prebuilt binary from GitHub Releases
   - Submit to AUR, maintain it or let a volunteer take over

4. **nixpkgs** — submit PR once project is polished enough to get merged
   - High bar: must be stable, well-documented, useful to general public
   - Once merged: `nix-env -iA nixpkgs.githut` or `nix profile install nixpkgs#githut`
   - Flake users: already works via `nix run github:karimKandil0/githut`

5. **Homebrew** — write a Formula or tap once Mac users request it
   - Either submit to `homebrew-core` (high bar, needs stable releases)
   - Or host own tap: `brew tap karimKandil0/tap && brew install githut`

6. **Debian/RPM** — lowest priority, most bureaucratic
   - `.deb` for apt, `.rpm` for dnf
   - Most small projects skip official repos and just offer binary downloads
   - Nix and cargo cover most Linux users who care about terminal tools

### GitHub Actions CI skeleton (for when you set it up)

```yaml
# .github/workflows/release.yml
on:
  push:
    tags: ['v*']
jobs:
  build:
    strategy:
      matrix:
        target:
          - x86_64-unknown-linux-gnu
          - x86_64-apple-darwin
          - aarch64-apple-darwin
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}
      - run: cargo build --release --target ${{ matrix.target }}
      - uses: actions/upload-artifact@v4
```

## Tooling Available to the Agent

The following tools are available in this Claude Code session. Use them — don't
shell out or do manually what a tool handles better.

### File & Code Tools
- **Read** — read any file before editing it (required before Write/Edit)
- **Edit** — surgical edits to existing files, preferred over Write for modifications
- **Write** — create new files or full rewrites only
- **Glob** — find files by pattern (e.g. `src/**/*.rs`)
- **Grep** — search file contents by regex across the codebase
- **Bash** — run shell commands: `cargo build`, `cargo check`, `cargo fmt`, git, etc.

### Planning & Tracking
- **TaskCreate / TaskUpdate** — create and track tasks for the current session
  - Use these to break down each phase into steps
  - Mark tasks `in_progress` when starting, `completed` when done
- **EnterPlanMode / ExitPlanMode** — use for planning before writing code on complex tasks

### GitHub MCP
The `mcp__github__*` tools provide direct GitHub API access without shelling out.
Prefer these over `gh` CLI for GitHub operations during development:

| Tool | Use case |
|------|----------|
| `mcp__github__create_issue` | file bugs, TODOs, known issues |
| `mcp__github__get_issue` / `list_issues` | check existing issues |
| `mcp__github__create_pull_request` | open PRs for feature branches |
| `mcp__github__get_pull_request_status` | check CI on a PR |
| `mcp__github__push_files` | push file changes directly |
| `mcp__github__create_or_update_file` | update single files on remote |
| `mcp__github__search_code` | search the codebase on GitHub |
| `mcp__github__list_commits` | review recent commit history |

Repo: `karimKandil0/githut`

### Web Tools
- **WebSearch** — look up crate docs, Rust patterns, ratatui examples
- **WebFetch** — fetch a specific docs page or crate README

## Agent Workflow Rules

These rules are mandatory. Follow them throughout the entire project.

### Commit after every meaningful change
Commit frequently and atomically. Every feature, fix, refactor, or new file gets
its own commit. Do not batch unrelated changes into one commit.

Commit message format:
```
type: short description
```

Types: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`

Keep it short. No body. No scope suffix unless genuinely useful.

### Keep CLAUDE.md up to date
CLAUDE.md is the source of truth for this project. Update it when:
- A phase task is completed — check it off
- A new pattern or decision is established — document it
- A dependency is added or changed — update the Stack section
- The project structure changes — update the Project Structure section
- A bug or gotcha is discovered — add a note so future sessions don't repeat it

Commit CLAUDE.md updates alongside the code they document.

### Before starting any phase task
1. Read the relevant existing files first
2. Run `cargo check` to confirm current state compiles
3. Create a TaskCreate entry for the work
4. Then write code

### After completing any phase task
1. Run `cargo check` — must pass
2. Run `cargo fmt` — must pass
3. Mark the task completed in CLAUDE.md
4. Commit

## Code Style

- No unwrap() in anything but throwaway test code
- Prefer ? operator for error propagation
- Keep ui.rs purely rendering — no logic, no API calls
- Keep github.rs purely API — no TUI state
- App struct owns all state, passed as &mut to render and event functions
- Async where needed (API calls), sync for rendering
