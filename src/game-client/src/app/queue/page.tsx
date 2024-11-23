/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useCallback, useEffect, useState } from "react";

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
  const [inQueue, setInQueue] = useState<boolean>(false);
  const [messages, setMessages] = useState<string[]>([]);
  const [socket, setSocket] = useState<WebSocket | null>(null);

  useEffect(() => {
    if (!inQueue) {
      console.log("Left queue");
      return;
    }
    console.log("Joined queue!");

    // Try to join websocket
    // BUG: Is it possible to already have a socket established?
    const webSocket = new WebSocket("ws://localhost:3001");
    if (webSocket == null) {
      console.error("Socket is null");
      return;
    }

    // Send join queue message on open
    webSocket.onopen = () => {
      const payload = JSON.stringify({
        JoinQueue: {
          user_id: user_id,
        },
      } satisfies JoinQueue);
      console.log("Sent " + payload);
      webSocket.send(payload);
    };
    webSocket.onerror = (event: Event) => {
      console.error("Failed to establish websocket: " + event);
      // TODO: retry
    };

    // Add message listener
    webSocket.onmessage = (event) => {
      setMessages((previous) => [...previous, event.data]);
    };

    // Close listener
    webSocket.onclose = (event) => {
      console.log("Closed websocket " + user_id);
      setMessages([])
    };

    setSocket(webSocket);
  }, [user_id, inQueue]);

  function joinQueue() {
    setInQueue(true);
  }

  function leaveQueue() {
    setInQueue(false);
    if (socket) {
      socket.close();
    }
  }

  return (
    <div className="m-2 space-y-4">
      <div className="space-x-10">
        <button className="bg-blue-50 text-black" onClick={joinQueue}>
          Connect
        </button>
        {inQueue && (
          <button className="bg-blue-50 text-black" onClick={leaveQueue}>
            Exit
          </button>
        )}
      </div>

      {inQueue && messages.length > 0 && (
        <div className="mt-4">
          <h3 className="text-lg font-semibold mb-2">Messages:</h3>
          <div className="bg-gray-100 p-4 rounded-lg max-h-60 overflow-y-auto">
            {messages.map((message, index) => (
              <div key={index} className="mb-2 p-2 bg-white rounded shadow">
                <pre className="whitespace-pre-wrap break-words">
                  {JSON.stringify(JSON.parse(message), null, 2)}
                </pre>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
