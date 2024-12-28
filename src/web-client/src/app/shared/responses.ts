// Response enum
export type MatchmakingResponse =
    | { type: "Connected"; userId: string }
    | { type: "AckJoinQueue" }
    | { type: "JoinedQueue" }
    | { type: "QueuePing"; timeElapsed: number }
    | { type: "MatchFound"; game_id: string; server_address: string }
    | { type: "JoinServer"; server_address: string }
    | { type: "Error"; message: string };
