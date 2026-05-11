# githut

a TUI for finding, browsing, and grabbing GitHub repos — without touching a browser.

search GitHub, read READMEs, browse file trees, clone, sparse-clone, fork, star.
all from the terminal, fully keyboard-driven.

built for headless environments, SSH sessions, and people who just prefer the terminal.

built with Rust + ratatui.

## features

- search GitHub repos with live README preview (auto-loads, 300ms debounce)
- language filter — cycle with Tab (`[Rust]`, `[Go]`, `[Python]`, etc.)
- browse repo file trees — navigate dirs, preview files (large file fallback via raw URL)
- clone repos to a local path (background, non-blocking)
- sparse clone — fetch only specific directories (`--filter=blob:none --sparse`)
- save individual files from a repo to disk
- star / unstar repos
- fork repos
- open repos in browser
- **my repos tab** — list, browse, clone your own repos
  - rename, delete, archive/unarchive
  - auto-loads your profile README on open (if `<user>/<user>` repo exists)
- rate limit display in status bar
- config at `~/.config/githut/config.toml` (`default_clone_path`)

## usage

```
githut
```

requires `gh` installed and authenticated:

```
gh auth login
```

## keybindings

| key       | action                                           |
|-----------|--------------------------------------------------|
| `1` / `2` | switch tabs (search / my repos)                  |
| `/`       | focus search                                     |
| `Enter`   | confirm search                                   |
| `Tab`     | cycle language filter                            |
| `j` / `k` | navigate list                                    |
| `J` / `K` | scroll preview pane                              |
| `l`       | open file browser / enter dir / preview file     |
| `h`       | go up one dir; at root, back to repo list        |
| `c`       | clone repo (browsing) / save file (file browser) |
| `C`       | sparse clone — prompts path + dirs               |
| `s`       | star / unstar                                    |
| `f`       | fork                                             |
| `o`       | open in browser                                  |
| `r`       | refresh results                                  |
| `R`       | rename repo (my repos tab)                       |
| `D`       | delete repo (my repos tab, asks y/n)             |
| `A`       | archive / unarchive (my repos tab)               |
| `?`       | help overlay                                     |
| `Esc`     | back / close overlay                             |
| `q`       | quit                                             |

## install

```
cargo install githut
```

or build from source inside the nix dev shell:

```
nix develop
cargo build --release
```
