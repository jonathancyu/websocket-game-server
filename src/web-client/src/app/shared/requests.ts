
export type SocketRequest<T> = {
    userId: string | null;
    request: T;
};

