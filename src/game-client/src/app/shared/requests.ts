// Requests
export type JoinQueue = { JoinQueue: null };
export type Ping = { Ping: null };
export type MatchmakingRequest = JoinQueue | Ping;
export type SocketRequest = {
    userId: string | null;
    request: MatchmakingRequest;
};

export const requestFactory = {
    connect: (userId: string | null) =>
    ({
        userId: userId,
        request: { Connect: null } satisfies Connect
    }),
    joinQueue: (userId: string | null) =>
    ({
        userId: userId,
        request: { JoinQueue: null } satisfies JoinQueue,
    }),
    ping: (userId: string) =>
    ({
        userId: userId,
        request: { Ping: null }
    }),
};
