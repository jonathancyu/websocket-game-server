# AGENTS.md

## Cursor Cloud specific instructions

This repo is a Rust Cargo workspace implementing a multiplayer rock-paper-scissors
game, plus a Next.js web client. Build/lint/test/run commands are standard and
documented below only where non-obvious.

### Services

| Service | Path | Run command (from repo root) | Ports | Notes |
| --- | --- | --- | --- | --- |
| game-server | `src/game-server` | `cargo run -p game-server` | REST `8082`, game WS `3002` | See blocking bug below — standalone binary panics on startup. |
| matchmaking-server | `src/matchmaking-server` | `cargo run -p matchmaking-server` | queue WS `3001`, REST `8081` | Opens `matchmaking.db` (SQLite) in the current working dir; calls game-server at `http://0.0.0.0:8082`. |
| web-client | `src/web-client` | `pnpm -C src/web-client dev` | HTTP `3030` | Next.js dev server. Connects to matchmaking WS at `ws://localhost:3001` (hardcoded). |
| agent | `src/agent` | `cargo run -p agent` | — | Stub, `main()` is empty. |

`src/common` is a shared library, not a runnable service. All addresses are
hardcoded in the binaries' `main.rs` (no env-var config).

### Lint / test / build

- Rust: `cargo build`, `cargo clippy`, `cargo test` from the repo root (workspace-wide). The `cargo test` suite stands up real game + matchmaking servers and plays a full game end-to-end; tests auto-create a temp SQLite DB from `sql/create_tables.sql`.
- Web client: `pnpm -C src/web-client lint` and `pnpm -C src/web-client build`.

### Required runtime setup (non-obvious)

- The matchmaking-server does NOT auto-create its DB schema. Before running it (not needed for `cargo test`), initialize the DB in the repo root:
  `sqlite3 matchmaking.db < sql/create_tables.sql`
  The file `matchmaking.db` is gitignored.
- Start order for a full live run: game-server (`8082`/`3002`) → matchmaking-server (`3001`/`8081`) → web-client (`3030`).
- System packages required to build the Rust crates: `libsqlite3-dev` (for `rusqlite`, which links system SQLite) and `libssl-dev` (for `openssl-sys`). These are installed in the VM snapshot, not by the update script.

### KNOWN BLOCKING BUG: game-server standalone binary panics

`src/game-server/src/entrypoint.rs` `serve()` calls `ready_signal.unwrap()`
unconditionally, but `src/game-server/src/main.rs` passes `None`, so
`cargo run -p game-server` panics immediately with
`called \`Option::unwrap()\` on a \`None\` value`. (The sibling
`matchmaking-server` `serve()` correctly guards this with `if let Some(...)`.)

Consequences:
- The game-server binary cannot run as-is, so a live full match cannot complete in the browser.
- When two players match, matchmaking-server POSTs to game-server `/create_game`; with game-server down this fails with `Connection refused` and the matchmaking-server's matchmaking task then panics (`matchmaking.rs` uses `.expect(...)`). The matchmaking-server stays up only while no two players match.

The fix is a one-liner mirroring matchmaking-server: replace
`ready_signal.unwrap().send(())...` with
`if let Some(ready_signal) = ready_signal { ready_signal.send(()).expect(...); }`.
This was intentionally NOT changed during environment setup (no app-code edits);
the workspace test suite (`cargo test`) exercises the full game flow regardless,
because the test path passes `Some(ready_sender)`.
