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

type GetServer = {
  GetServer: {
    user_id: string;
  };
};
type JoinQueue = {
  JoinQueue: {
    user_id: string;
  };
};

export default function Queue() {
  const [user_id] = useState(() => crypto.randomUUID());
  const [socketHandle, setSocketHandle] = useState<SocketHandle | null>(null);

  function connect() {
    const { messages, sendMessage } = useWebSocket("ws://localhost:3001");
    setSocketHandle({
      messages: messages,
      sendMessage: sendMessage,
    });
  }

  function getServer() {
    if (socketHandle == null) {
      console.log("Socket handle null");
      return;
    }
    const request: GetServer = {
      GetServer: {
        user_id: user_id,
      },
    };
    const payload = JSON.stringify(request);
    socketHandle.sendMessage(payload);
    console.log("Sent " + payload);
  }

  function joinQueue() {
    if (socketHandle == null) {
      console.log("Socket handle null");
      return;
    }
    const request: JoinQueue = {
      JoinQueue: {
        user_id: user_id,
      },
    };
    const payload = JSON.stringify(request);
    socketHandle.sendMessage(payload);
    console.log("Sent " + payload);
  }

  return (
    <div className="m-2 space-x-10">
      <button className="bg-blue-50 text-black" onClick={connect}>
        Connect
      </button>
      <button className="bg-blue-50 text-black" onClick={joinQueue}>
        Join queue
      </button>
      <button className="bg-blue-50 text-black" onClick={getServer}>
        get server
      </button>
      {socketHandle && (
        <div>
          {socketHandle.messages.map((msg, index) => (
            <div key={index}>{msg}</div>
          ))}
        </div>
      )}
    </div>
  );
}
