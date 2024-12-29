import { Move, Result } from "./model";

export type GameRequest =
    | { type: "JoinGame" }
    | { type: "Move"; move: Move };

export type GameResponse =
    | { type: "GameJoined" }
    | { type: "PendingMove" }
    | { type: "RoundResult"; result: Result; other_move: Move }
    | { type: "Error"; result: Result, wins: number, total: number };
