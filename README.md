# githut

a TUI for finding, browsing, and grabbing GitHub repos — without touching a browser.

search GitHub, read READMEs, browse file trees, clone, sparse-clone, fork, star.
all from the terminal, fully keyboard-driven.

built for headless environments, SSH sessions, and people who just prefer the terminal.

built with Rust + ratatui.

> BUILT WITH THE HELP OF CLAUDE CODE (i feel like this needs to be said aloud.)



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
- **profile view** — browse any user's public repos and bio (`u` on any repo)
- **issues & PRs** — browse, filter, create, and close issues for any repo (`i`)
- **notifications** — view and mark GitHub notifications read (`3`)
- **code search** — search code within any repo (`S`)
- **create repos** — create new repos from the TUI (`n` in My Repos)
- **topic search** — prefix query with `#` to search by topic (`#rust`, `#neovim`)
- **search history** — Up/Down in search cycles previous queries
- **recently viewed** — shows last 10 repos when search is empty
- **fuzzy filter** — type in browse mode to filter current results client-side
- **social** — follow/unfollow users (`F`), view followers/following (`W`/`E`)
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

### global

| key           | action                          |
|---------------|---------------------------------|
| `1` / `2`     | switch tabs (search / my repos) |
| `3`           | notifications                   |
| `?`           | help overlay                    |
| `Esc`         | back / close overlay            |
| `q`           | quit                            |

### search & browsing

| key       | action                                           |
|-----------|--------------------------------------------------|
| `/`       | focus search                                     |
| `Enter`   | confirm search                                   |
| `Tab`     | cycle language filter                            |
| `r`       | refresh results                                  |
| `j` / `k` | navigate list                                    |
| `J` / `K` | scroll README preview                            |
| `l`       | open file browser                                |
| `u`       | view repo owner's profile                        |
| `i`       | browse issues / PRs                              |
| `s`       | star / unstar                                    |
| `f`       | fork                                             |
| `c`       | clone repo                                       |
| `C`       | sparse clone — prompts path + dirs               |
| `o`       | open in browser                                  |

### file browser

| key       | action                                           |
|-----------|--------------------------------------------------|
| `j` / `k` | navigate                                         |
| `J` / `K` | scroll file preview                              |
| `l`       | enter dir / preview file                         |
| `h`       | go up one dir; at root, back to repo list        |
| `c`       | save file to local path                          |

### my repos (tab 2)

| key   | action                        |
|-------|-------------------------------|
| `R`   | rename repo                   |
| `D`   | delete repo (confirms y/n)    |
| `A`   | archive / unarchive           |
| `n`   | create new repo               |
| `S`   | code search within repo       |

### profile view (`u` on any repo)

| key   | action                        |
|-------|-------------------------------|
| `j/k` | navigate repos                |
| `u`   | go to that repo's owner       |
| `F`   | follow / unfollow user        |
| `W`   | view followers list           |
| `E`   | view following list           |
| `o`   | open profile in browser       |
| `h`   | back                          |

### issues & PRs (`i` on any repo)

| key     | action                              |
|---------|-------------------------------------|
| `j` / `k` | navigate list                     |
| `J` / `K` | scroll issue preview              |
| `l`     | open issue + comments               |
| `Tab`   | toggle issues / pull requests       |
| `f`     | cycle filter: open → closed → all   |
| `n`     | create new issue                    |
| `x`     | close selected issue                |
| `o`     | open in browser                     |
| `h`     | back to repo list                   |

### notifications (tab 3)

| key   | action                        |
|-------|-------------------------------|
| `j/k` | navigate                      |
| `r`   | mark selected as read         |
| `R`   | mark all as read              |
| `f`   | toggle unread-only filter     |
| `o`   | open in browser               |
| `h`   | back                          |

## install

```
cargo install githut
```

or grab a prebuilt binary from [releases](https://github.com/karimKandil0/githut/releases).

## config

optional config at `~/.config/githut/config.toml`:

```toml
# default path prefix for cloning repos
# clone of foo/bar goes to ~/repos/bar
default_clone_path = "~/repos"
```

## contributing

see [CONTRIBUTING.md](CONTRIBUTING.md).
