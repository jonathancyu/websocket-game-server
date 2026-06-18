# Project Features Summary

This document summarizes all features present in the **websocket-game-server** project as of the current codebase. The project is an exploration of Rust WebSockets and real-time 1v1 matchmaking for Rock-Paper-Scissors.

## Project Overview

| Aspect | Details |
|--------|---------|
| **Purpose** | Learn Rust + WebSockets; build a matchmaking → game → client flow |
| **Game** | Rock-Paper-Scissors, best-of-N rounds (`games_to_win`) |
| **Architecture** | Shared Rust library + 3 backend crates + Next.js frontend |
| **Workspace crates** | `common`, `matchmaking-server`, `game-server`, `agent` |
| **Frontend** | Next.js 15 + React 19 web client ("Matchmaking Tester") |

### Roadmap Status (from README)

| Item | Status |
|------|--------|
| Naive 1-1 matchmaking | Done |
| Rock-Paper-Scissors socket server | Done |
| Rock-Paper-Scissors web client | Done |
| Deploy & stress test | Partial (multi-client UI exists; no load tooling) |
| Autonomous game agents | Partial (strategy skeleton only) |
| ELO matchmaking | Not started |

---

## Component Features

### 1. `common` — Shared Library

Shared types, WebSocket framework, utilities, and JSON-driven integration test harness used by all Rust services.

**Features:**
- **`Id` type** — UUID wrapper for players and games
- **Game model** — `Move` (Rock/Paper/Scissors), `Outcome` (Win/Loss/Draw), and `Move::beats()` logic
- **Cross-service REST DTOs** — `CreateGameRequest/Response`, `GetGameRequest/Response`, `PostGameResultsRequest`
- **`WebsocketHandler` trait** — Generic async WebSocket server: bind listener, accept connections, require first message as `OpenSocketRequest { userId }`, route JSON tagged messages, push responses to clients, optional connection close after certain responses
- **Utilities** — Graceful shutdown on Ctrl+C/SIGTERM, random ephemeral port binding, URL helpers
- **`TestCase` harness** — Loads JSON test sequences (socket open/send/receive, HTTP POST) with `${placeholder}` substitution and UUID normalization for assertions

---

### 2. `matchmaking-server` — Queue & Pairing

Accepts players into a queue over WebSocket, pairs them FIFO, creates DB records, provisions games on the game server, and notifies matched players.

**Default ports:**
- WebSocket queue: `0.0.0.0:3001`
- REST API: `0.0.0.0:8081`
- Game server URL: `http://0.0.0.0:8082`
- SQLite DB: `matchmaking.db`

**Features:**
- Dual-thread architecture: `QueueSocket` (WebSocket) + `MatchmakingService` (queue polling + REST)
- **Naive 1-1 FIFO matchmaking** — polls every 50ms, pairs consecutive players
- Duplicate queue join prevention via `users_in_queue` set
- On match: inserts `match` row, POSTs to game server `/create_game`, pushes `MatchFound` to both players
- **`games_to_win` hardcoded to 1** when creating games
- SQLite persistence for matches and (partially) results
- Graceful shutdown via broadcast channel

**HTTP endpoints:**

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/` | Health/hello |
| `POST` | `/game/result` | Accept finished-game results |

**WebSocket protocol (after `userId` handshake):**

| Direction | Message | Purpose |
|-----------|---------|---------|
| C→S | `JoinQueue` | Enter matchmaking queue |
| C→S | `Ping` | Keepalive (returns stub `QueuePing { time_elapsed: 0 }`) |
| C→S | `GetServer` | Reconnect helper (stub — returns IPv6 `::`) |
| S→C | `JoinedQueue` | Confirms queue entry |
| S→C | `QueuePing` | Ping response |
| S→C | `MatchFound` | `{ game_id, server_address }` — closes WebSocket |
| S→C | `JoinServer` | Stub reconnect response — closes WebSocket |

**Gaps:**
- ELO update is TODO (`write_game_result_and_update_elo` only inserts scores)
- Game server does not POST results back to matchmaking
- No skill-based pairing
- `LeaveQueue` handled internally but not exposed via WebSocket

---

### 3. `game-server` — Rock-Paper-Scissors Game Host

Hosts individual RPS games. REST manager creates games and spawns per-game Tokio tasks; WebSocket layer routes player messages to the correct game thread.

**Default ports:**
- REST manager: `0.0.0.0:8082`
- Game WebSocket: `0.0.0.0:3002`

**Features:**
- **GameManager** — Axum REST + router thread + player→game assignment map
- **One Tokio task per game** (`GameThread::thread_loop`)
- **GameSocket** — Implements `WebsocketHandler`; forwards client messages to game manager via MPSC channel
- Player conflict detection: returns `409 CONFLICT` if either player already assigned to a game
- Closes WebSocket after `MatchResult` is sent
- HTTP tracing via `tower-http::TraceLayer`

**HTTP endpoints:**

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/` | Health/hello |
| `POST` | `/create_game` | Create game → `201` + `{ game_id, address }` |
| `GET` | `/game/{game_id}` | Lookup game players |

**WebSocket protocol (after `userId` handshake):**

| Direction | Message | Purpose |
|-----------|---------|---------|
| C→S | `JoinGame` | Join the game room |
| C→S | `Move` | Submit `{ value: Rock \| Paper \| Scissors }` |
| S→C | `GameJoined` | Confirms join |
| S→C | `PendingMove` | Both players ready — submit moves |
| S→C | `RoundResult` | `{ result: Win/Loss, other_move }` |
| S→C | `MatchResult` | `{ result, wins, total }` — closes WebSocket |

**Game state machine:**
1. `WaitingForPlayers` — collect `JoinGame`; when 2 connected → send `PendingMove` to both
2. `PendingMoves` — collect moves; when both submitted → evaluate round
3. `Done` — match over; thread exits

**Game mechanics:**
- Standard RPS rules via `Move::beats()`
- First to `games_to_win` round wins wins the match
- Draws increment `rounds_played` but do not send `RoundResult` (silent reset)
- Matchmaking currently sets `games_to_win = 1` (single round wins match)

**Gaps:**
- Does not report results to matchmaking server
- No draw notifications to clients
- No rematch / return-to-queue automation

---

### 4. `agent` — Bot / Stress-Test Layer

Intended bot/automation layer for stress testing and strategy experiments.

**Current state: skeleton only**
- `main.rs` is empty — no runnable agent
- `Client` struct holds strategy + history + `play()` method (not wired to network)
- **Strategies implemented:**
  - `OnlyRock`, `OnlyPaper`, `OnlyScissors` — static moves
  - `RandomMove` — stub (always returns Rock)

Dependencies (`tokio`, `tokio-tungstenite`, `reqwest`) suggest future queue/game automation but are not implemented.

---

### 5. `web-client` — Matchmaking Tester UI

Next.js 15 + React 19 RC frontend for spawning multiple simulated clients, queuing for matches, playing RPS, and tracking W/L records.

**Dev server:** port `3030` (`pnpm dev`)

**Features:**
- **Multi-client stress UI** (`ClientList`) — add/remove clients with random UUIDs; responsive grid layout
- **Per-client state machine** (`Client`) — `queue` ↔ `game` screens
- **Queue flow** (`Queue`) — connect to `ws://localhost:3001`, join queue, show message log, leave queue
- **Game flow** (`Game`) — connect to game server from `MatchFound`, join game, Rock/Paper/Scissors buttons, round/match result display
- **Record tracking** — per-client W/L (draws count as 0.5 each)
- **Reusable `useWebSocket` hook** — sends `{ userId }` on open, auto-reconnect on abnormal close (5s delay)
- **Styling** — Tailwind CSS, dark stone/slate theme, Geist fonts

**Hardcoded assumptions:**
- Matchmaking WebSocket: `ws://localhost:3001`
- Game server address comes dynamically from `MatchFound.server_address`

---

## End-to-End Flow

```
Web Client → Matchmaking WS (3001) → JoinQueue → JoinedQueue
         → FIFO pair (50ms poll) → INSERT match → POST /create_game
         → MatchFound → Game WS (3002) → JoinGame → PendingMove
         → Move → RoundResult / MatchResult → return to queue in UI
```

Sequence diagrams are maintained in:
- `docs/matchmaking_sequence.puml`
- `docs/game_sequence.puml`

---

## Database Schema

SQLite (`matchmaking.db`), defined in `sql/create_tables.sql`:

**`match`**

| Column | Type | Purpose |
|--------|------|---------|
| `id` | TEXT PK | Match/game UUID |
| `player_1_id` | TEXT | First player |
| `player_2_id` | TEXT | Second player |
| `games_to_win` | INTEGER | Target wins |
| `start_time` | TIMESTAMP | Default `CURRENT_TIMESTAMP` |

**`match_results`**

| Column | Type | Purpose |
|--------|------|---------|
| `id` | TEXT PK | Same as match id |
| `player_1_score` | INTEGER | Final round wins for player 1 |
| `player_2_score` | INTEGER | Final round wins for player 2 |
| `end_time` | TIMESTAMP | Default `CURRENT_TIMESTAMP` |

**Usage today:**
- `match` rows inserted on pairing
- `match_results` inserted only if something POSTs to `/game/result` (game server doesn't)
- No ELO/player tables

---

## Testing Infrastructure

**Rust integration tests (JSON-driven):**
- `game-server/test/data/full_game.json` — full 2-round match via REST create + dual WebSocket clients
- `matchmaking-server/test/data/queue_multiple_times.json` — two players queue and receive `MatchFound`

**Test harness capabilities:**
- Multi-endpoint orchestration (named WebSocket + REST clients)
- `${placeholder}` substitution
- UUID-agnostic assertions via regex replacement

---

## Deployment / Infrastructure

**Dockerfile (multi-stage):**
- Build stage: `rust:1.85.0`, `cargo build --release`
- Runtime targets: `game-server`, `matchmaking-server`
- No web-client Docker image
- No docker-compose or orchestration manifests

**Runtime requirements:**
- Run matchmaking-server (needs SQLite file path)
- Run game-server (matchmaking must reach its REST API)
- Run web-client separately (`pnpm dev` on port 3030)

**Observability:**
- `tracing` + `tracing-subscriber` at DEBUG level
- HTTP request tracing on Axum routes

**Graceful shutdown:**
- Ctrl+C / SIGTERM → broadcast shutdown to all spawned tasks

---

## Architecture Patterns

| Pattern | Where |
|---------|-------|
| Tagged JSON enums (`serde tag = "type"`) | All WS protocols |
| MPSC channels between WS layer and business logic | Both servers |
| Per-connection Tokio task | `common::websocket` |
| Per-game Tokio task | `game-server` |
| Shared mutex state | Queue + game manager maps |
| Broadcast shutdown | All services |
| Generic `WebsocketHandler` trait | Extensible WS servers |

---

## Feature Maturity Summary

| Feature | Status |
|---------|--------|
| Naive FIFO 1v1 matchmaking | Implemented |
| Rock-Paper-Scissors game server | Implemented |
| Multi-client web UI | Implemented |
| JSON integration test harness | Implemented |
| Docker images for Rust servers | Partial |
| Deploy & stress test | Partial |
| Game result callback to matchmaking | Endpoint exists; not wired |
| ELO matchmaking / rating updates | Roadmap + TODO |
| Autonomous game agents/bots | Strategy skeleton only |
| Queue ping with real elapsed time | Returns 0 |
| Draw round client notification | Silent reset |
| Reconnect via `GetServer` | Stub |

---

## Primary Demo Path

1. Start `matchmaking-server` and `game-server`
2. Run `pnpm dev` in `src/web-client`
3. Add 2+ clients in the UI
4. Click **Join Queue** on each client
5. When matched, play Rock-Paper-Scissors
6. After match ends, clients return to queue for another round
