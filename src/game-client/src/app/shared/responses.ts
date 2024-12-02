
// Response enum
export interface Connected {
    kind: "Connected";
    Connected: { userId: string };
}
export interface AckJoinQueue {
    kind: "AckJoinQueue";
    AckJoinQueue: null;
}
export interface JoinedQueue {
    kind: "JoinedQueue";
    JoinedQueue: null;
}
export interface QueuePing {
    kind: "QueuePing";
    QueuePing: { timeElapsed: number };
}
export interface MatchFound {
    kind: "MatchFound";
    MatchFound: { game_id: string, server_address: string };
}
export interface JoinServer {
    kind: "JoinServer";
    JoinServer: { server_address: string };
}
export interface Error {
    kind: "Error";
    Error: { message: string };
}


// TODO: Need to abstract this out
export type MatchmakingResponse =
    | Connected
    | AckJoinQueue
    | JoinedQueue
    | QueuePing
    | MatchFound
    | JoinServer
    | Error;


