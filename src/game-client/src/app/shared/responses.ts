
// Response enum
export interface Connected {
    kind: "Connected";
    Connected: { userId: string };
}
export interface JoinedQueue {
    kind: "JoinedQueue";
    JoinedQueue: null;
}
export interface QueuePing {
    kind: "QueuePing";
    QueuePing: { timeElapsed: number };
}
export interface JoinServer {
    kind: "JoinServer";
    JoinServer: { serverIp: string };
}
export interface Error {
    kind: "Error";
    Error: { message: string };
}


// TODO: Need to abstract this out
export type MatchmakingResponse =
    | Connected
    | JoinedQueue
    | QueuePing
    | JoinServer
    | Error;

export type SocketResponse<T> = {
    userId: string | null;
    message: T;
};

