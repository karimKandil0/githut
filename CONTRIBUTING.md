# contributing to githut

thanks for wanting to help. here's what you need to know.

## setup

you need:
- Rust (stable)
- `gh` installed and authenticated (`gh auth login`)
- optionally: Nix (the dev shell sets up all env vars correctly for NixOS)

```
git clone https://github.com/karimKandil0/githut
cd githut

# with nix
nix develop
cargo run

# without nix (set these manually if on NixOS or linking fails)
# LIBGIT2_SYS_USE_PKG_CONFIG=1 OPENSSL_NO_VENDOR=1 cargo run
cargo run
```

## project layout

```
src/
  main.rs        -- entrypoint, event loop
  app.rs         -- App struct, all state
  types.rs       -- shared types (Repo, AppState, etc.)
  config.rs      -- config file + search history
  git.rs         -- clone, sparse-clone
  input.rs       -- TextInput widget (cursor, history)
  markdown.rs    -- markdown -> ratatui Lines renderer
  api/
    auth.rs      -- shells out to `gh auth token`
    client.rs    -- all GitHub API calls
  tui/
    ui.rs        -- all rendering (ratatui)
    events.rs    -- all keyboard handling
```

key rule: **ui.rs is purely rendering — no logic, no API calls**. all state lives in `App`.

## making changes

1. fork + branch
2. make your change
3. `cargo check` and `cargo fmt` must pass
4. open a PR — describe what and why

## adding a feature

- new state goes in `types.rs` as an `AppState` variant
- new app fields go in `app.rs`
- API calls go in `api/client.rs`
- keyboard handling goes in `tui/events.rs`
- rendering goes in `tui/ui.rs`

## commit style

```
type: short description
```

types: `feat`, `fix`, `refactor`, `chore`, `docs`

keep it short. no body needed for small changes.

## reporting bugs

open an issue. include:
- what you did
- what you expected
- what happened instead
- terminal + OS

## note

this project was built with the help of Claude Code. contributions from humans are very welcome.
