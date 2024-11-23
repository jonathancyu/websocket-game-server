/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useEffect, useState } from "react";

type SocketHandle = {
  messages: string[];
  sendMessage: (message: string) => void;
};

const useWebSocket = (url: string) => {
  const [messages, setMessages] = useState<string[]>([]);
  const [ws, setWs] = useState<WebSocket | null>(null);

  useEffect(() => {
    const socket = new WebSocket(url);
    setWs(socket);

    socket.onmessage = (event) => {
      setMessages((previous) => [...previous, event.data]);
    };

    return () => {
      socket.close();
    };
  }, [url]);

  const sendMessage = (message: string) => {
    if (ws) {
      ws.send(message);
    }
  };

  return { messages, sendMessage };
};

// type GetServer = {
//   GetServer: {
//     user_id: string;
//   };
// };
type JoinQueue = {
  JoinQueue: {
    user_id: string;
  };
};

export default function Queue() {
  const [user_id] = useState(() => crypto.randomUUID());
  // const [socketHandle, setSocketHandle] = useState<SocketHandle | null>(null);
  // const { messages, sendMessage } = useWebSocket("ws://localhost:3001");
  const [inQueue, setInQueue] = useState<boolean>(false);
  const [queueSocket, setQueueSocket] = useState<WebSocket | null>(null);

  // function joinQueue() {
  //   if (socketHandle == null) {
  //     console.log("Socket handle null");
  //     return;
  //   }
  //   const request: JoinQueue = {
  //     JoinQueue: {
  //       user_id: user_id,
  //     },
  //   };
  //   const payload = JSON.stringify(request);
  //   socketHandle.sendMessage(payload);
  //   console.log("Sent " + payload);
  useEffect(() => {
    if (inQueue) {
      if (queueSocket !== null) {
        // Queue websocket is already established, do nothing
        return;
      }
      // Try to join queue

      try {
        const socket = new WebSocket("ws://localhost:3001");

        socket.onerror = (error) => {
          console.error("WebSocket error:", error);
          setQueueSocket(null);
        };

        socket.onclose = () => {
          console.log("WebSocket connection closed");
          setQueueSocket(null);
        };

        socket.onopen = () => {
          console.log("WebSocket connection established");
        };
      } catch (error) {
        console.error("Failed to create WebSocket connection:", error);
        setInQueue(false);
        setQueueSocket(null);
      }
    }
  }, [queueSocket, inQueue]);

  function joinQueue() {
    const request: JoinQueue = {
      JoinQueue: {
        user_id: user_id,
      },
    };
    const payload = JSON.stringify(request);
    socketHandle.sendMessage(payload);
    console.log("Sent " + payload);
  }
  function connect() {
    setInQueue(true);
  }

  return (
    <div className="m-2 space-x-10">
      <button className="bg-blue-50 text-black" onClick={connect}>
        Connect
      </button>
    </div>
  );
}
