/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useEffect, useState } from "react";


const MatchmakingRequest = {
  joinQueue: () => ({ JoinQueue: null }),
  ping: () => ({ Ping: null }),
} as const;

export default function Client() {
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
      const payload = JSON.stringify(MatchmakingRequest.joinQueue());
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
      setMessages([]);
      setInQueue(false);
    };

    // Set up polling interval
    const pollInterval = setInterval(() => {
      if (webSocket.readyState == WebSocket.OPEN) {
        const payload = JSON.stringify(MatchmakingRequest.ping());
        console.log("Sent ping: " + payload);
        webSocket.send(payload);
      }
    }, 5000);

    setSocket(webSocket);

    return () => {
      clearInterval(pollInterval);
      webSocket.close();
    };
  }, [user_id, inQueue]);

  function joinQueue() {
    setInQueue(true);
  }

  function leaveQueue() {
    setInQueue(false);
    setMessages([]);
    if (socket) {
      socket.close();
    }
  }

  return (
    <div className="m-2 space-y-4">
      <div className="space-x-4">
        <button
          className="px-6 py-2 rounded-md bg-blue-50 text-black border-2 border-blue-200 hover:bg-blue-100 transition-colors duration-200 font-medium shadow-sm"
          onClick={joinQueue}
        >
          Join Queue
        </button>
        {inQueue && (
          <button
            className="px-6 py-2 rounded-md bg-red-50 text-red-700 border-2 border-red-200 hover:bg-red-100 transition-colors duration-200 font-medium shadow-sm"
            onClick={leaveQueue}
          >
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
