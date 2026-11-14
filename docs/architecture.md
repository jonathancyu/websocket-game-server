# Architecture

## Components

1. **Matchmaking server** accepts websocket connections, tracks queued players and emits `MatchFound` once two players are paired.
2. **Game server** hosts one game per pair and resolves each round using `Move::beats`.
3. **Agents** connect like any other client and choose moves via a `Strategy`.
4. **Web client** drives the flow manually for testing.

## Flow

See `matchmaking_sequence_diagram.puml` for the queue and match-found handshake.

## Known gaps

- `RandomMove` always plays Rock.
- No persistence of finished games.
- No authentication on websocket connections.
