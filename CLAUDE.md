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
- Git operations: git2 (libgit2 bindings)
- Markdown rendering: termimad
- Error handling: anyhow
- Serialization: serde + serde_json

## Project Structure

```
githut/
  src/
    main.rs          -- entrypoint, boots tokio runtime, initializes terminal, runs app
    app.rs           -- App struct, central state machine
    ui.rs            -- all ratatui rendering logic
    events.rs        -- keyboard input handling, event loop
    github.rs        -- GitHub API calls (search, fork, star, get readme)
    git.rs           -- local git operations (clone, sparse-clone)
    auth.rs          -- auth: shells out to `gh auth token` to get token
    types.rs         -- shared types (Repo, SearchResult, AppState, etc.)
  Cargo.toml
  flake.nix
  CLAUDE.md
```

## Layout

```
+----------------------------------+-----------------------------+
| [ search: ______________ ] [lang] |                             |
+----------------------------------+  README preview              |
| owner/repo-name      Rust  4.2k  |  (rendered markdown via      |
| short description here...        |   termimad)                  |
| > owner/selected-repo  Go  891   |                             |
|   description of selected...     |                             |
|                                  |                             |
+----------------------------------+-----------------------------+
 c:clone  f:fork  s:star  o:browser  /:search  tab:filter  q:quit
```

Left pane: scrollable results list (50% width)
Right pane: README of currently highlighted repo (50% width)
Bottom bar: keybindings hint
Top: search input + optional language filter

## Keybindings

| Key       | Action                              |
|-----------|-------------------------------------|
| /         | Focus search input                  |
| Enter     | Confirm search / select             |
| j / k     | Move down / up in results           |
| tab       | Cycle language filter               |
| c         | Clone selected repo (prompts path)  |
| C         | Sparse-clone (prompts path + dirs)  |
| f         | Fork selected repo                  |
| s         | Star / unstar selected repo         |
| o         | Open selected repo in browser       |
| r         | Refresh / re-fetch results          |
| ?         | Toggle help overlay                 |
| Esc       | Clear input / close overlay         |
| q         | Quit                                |

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
- Fork repo: `POST /repos/{owner}/{repo}/forks`
- Star repo: `PUT /user/starred/{owner}/{repo}`
- Unstar repo: `DELETE /user/starred/{owner}/{repo}`
- Check if starred: `GET /user/starred/{owner}/{repo}` (204 = starred, 404 = not)

Rate limits: unauthenticated = 10 req/min search, authenticated = 30 req/min search.
Always use auth. Show rate limit remaining in status bar if possible.

## Git Operations

Clone uses git2:
```rust
// git.rs pattern
git2::Repository::clone(url, path)?;
```

Sparse clone flow:
1. Init repo (no checkout)
2. Add remote
3. Set sparse-checkout patterns
4. Pull only matching paths

## App State Machine

```
enum AppState {
    Searching,        // user is typing in search bar
    Browsing,         // navigating results list
    Previewing,       // scrolling README pane (focused)
    Cloning,          // clone path input prompt
    SparseCloning,    // sparse-clone path + dirs prompt
    Error(String),    // showing error overlay
    Help,             // showing help overlay
}
```

State transitions are handled in events.rs based on keypress + current state.

## Phases

### Phase 1 — Core (current focus)
Goal: working search + browse + clone. Shippable v0.1.

- [ ] auth.rs: `gh auth token` integration, error if missing
- [ ] github.rs: search repos, return Vec<Repo>
- [ ] types.rs: Repo struct, AppState enum, SearchResult
- [ ] app.rs: App struct with state, results, selected index, search query
- [ ] ui.rs: split layout, results list, search bar, keybindings bar
- [ ] events.rs: keyboard loop, j/k navigation, / for search
- [ ] github.rs: fetch README for selected repo
- [ ] ui.rs: README preview pane with termimad rendering
- [ ] git.rs: basic clone on `c` keypress with path prompt

### Phase 2 — GitHub Actions
Goal: fork, star, open browser.

- [ ] github.rs: fork endpoint
- [ ] github.rs: star / unstar + check if starred
- [ ] ui.rs: star indicator on repo in list (show if starred)
- [ ] events.rs: f, s, o keybindings
- [ ] open crate: browser open on `o`

### Phase 3 — Power Features
Goal: sparse clone, language filter, better UX.

- [ ] git.rs: sparse-clone implementation
- [ ] events.rs: C for sparse-clone, prompt for dirs
- [ ] ui.rs: language filter tab cycling
- [ ] github.rs: pass language filter to search query
- [ ] ui.rs: rate limit display in status bar
- [ ] ui.rs: help overlay on `?`
- [ ] config: ~/.config/githut/config.toml for default clone path, etc.

### Phase 4 — Your Repos (direction 2)
Goal: manage your own repos, not just discover.

- [ ] github.rs: list authenticated user's repos
- [ ] ui.rs: tab switching between Search and My Repos views
- [ ] github.rs: delete, archive, rename via API
- [ ] github.rs: list and sync forks

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

## Code Style

- No unwrap() in anything but throwaway test code
- Prefer ? operator for error propagation
- Keep ui.rs purely rendering — no logic, no API calls
- Keep github.rs purely API — no TUI state
- App struct owns all state, passed as &mut to render and event functions
- Async where needed (API calls), sync for rendering
