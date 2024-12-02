import { useState } from "react";
import { requestFactory } from "../shared/requests";

export enum ConnectionStatus {
  Off,
  Connecting,
  Connected,
  Failed,
}

export type SocketRequest<T> = {
  userId: string | null;
  request: T;
};
export type SocketResponse<T> = {
  userId: string | null;
  message: T;
};

// TODO: Can I not just make a class...???
export type SocketHook<RQ, RS> = {
  // Fields
  connectionStatus: ConnectionStatus;
  // Methods
  connect: (url: string, onmessage: (message: RS) => void) => void;
  send: (msg: RQ) => void;
  close: () => void;
};

export default function useWebSocket<RQ, RS>(): SocketHook<RQ, RS> {
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>(
    ConnectionStatus.Off,
  );
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [userId, setUserId] = useState<string | null>(null);

  const connectWebSocket = (url: string, onmessage: (response: RS) => void) => {
    const newSocket = new WebSocket(url);
    setConnectionStatus(ConnectionStatus.Connecting);

    newSocket.onopen = () => {
      const payload = JSON.stringify(requestFactory.joinQueue(userId));
      console.log("Sent " + payload);
      newSocket.send(payload);
      setConnectionStatus(ConnectionStatus.Connected);
    };

    newSocket.onerror = (event: Event) => {
      console.log("Error: ", event);
      // TODO: Retry logic?
      setConnectionStatus(ConnectionStatus.Failed);
    };

    newSocket.onmessage = (event) => {
      const message = JSON.parse(event.data) as SocketResponse<RS>;
      if (!userId) {
        setUserId(message.userId);
      }
      onmessage(message.message);
    };

    newSocket.onclose = (event: CloseEvent) => {
      console.log(connectionStatus);
      console.log(
        "Closed websocket with code " + event.wasClean + ":" + event.reason,
      );
      setSocket(null);
      setConnectionStatus(ConnectionStatus.Off);
    };

    setSocket(newSocket);
  };

  return {
    connectionStatus: connectionStatus,
    connect: connectWebSocket,
    send: (msg: RQ) => {
      if (socket) {
        socket.send(JSON.stringify(msg));
      }
    },
    close: () => {
      if (socket) {
        socket.close();
      }
    },
  };
}
