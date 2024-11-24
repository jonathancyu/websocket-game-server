/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useEffect, useState } from "react";

const MatchmakingRequest = {
  joinQueue: () => ({ JoinQueue: null }),
  ping: () => ({ Ping: null }),
} as const;

type ClientProps = {
  id: string;
};

enum ConnectionStatus {
  Off,
  Connecting,
  Connected,
  Failed,
}

export default function Client({ id }: ClientProps) {
  const [inQueue, setInQueue] = useState<boolean>(true);
  const [messages, setMessages] = useState<string[]>([]);
  const [socket, setSocket] = useState<WebSocket | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatus>(
    ConnectionStatus.Off,
  );

  useEffect(() => {
    if (!inQueue) {
      console.log("Left queue");
      return;
    }
    console.log("Joined queue!");

    // Try to join websocket
    const socket = new WebSocket("ws://localhost:3001");
    setConnectionStatus(ConnectionStatus.Connecting);
    // BUG: Is it possible to already have a socket established?
    if (socket == null) {
      console.error("Socket is null");
      return;
    }

    // Send join queue message on open
    socket.onopen = () => {
      const payload = JSON.stringify(MatchmakingRequest.joinQueue());
      console.log("Sent " + payload);
      socket.send(payload);
      setConnectionStatus(ConnectionStatus.Connected);
    };
    socket.onerror = (event: Event) => {
      console.log("Error: ", event);
      setConnectionStatus(ConnectionStatus.Failed);
    };

    // Add message listener
    socket.onmessage = (event) => {
      setMessages((previous) => [...previous, event.data]);
    };

    // Close listener
    socket.onclose = (event) => {
      console.log(connectionStatus);
      if (connectionStatus != ConnectionStatus.Failed) {
        console.log("Closed websocket " + id);
        setMessages([]);
        setInQueue(false);
      }
    };

    // Set up polling interval
    const pollInterval = setInterval(() => {
      if (socket.readyState == WebSocket.OPEN) {
        const payload = JSON.stringify(MatchmakingRequest.ping());
        console.log("Sent ping: " + payload);
        socket.send(payload);
      }
    }, 5000);

    setSocket(socket);

    return () => {
      clearInterval(pollInterval);
      socket.close();
    };
  }, [socket, id, inQueue]);

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
    <div className="m-2 space-y-4 text-black">
      <div className="space-x-4">
        {connectionStatus == ConnectionStatus.Off ||
          (connectionStatus == ConnectionStatus.Failed && (
            <button
              className="px-6 py-2 rounded-md bg-blue-50 text-black border-2 border-blue-200 hover:bg-blue-100 transition-colors duration-200 font-medium shadow-sm"
              onClick={joinQueue}
            >
              Join Queue
            </button>
          ))}
        {connectionStatus == ConnectionStatus.Connecting && (
          <span className="inline-flex items-center text-yellow-600">
            <svg className="animate-spin h-5 w-5 mr-2" viewBox="0 0 24 24">
              <circle
                className="opacity-25"
                cx="12"
                cy="12"
                r="10"
                stroke="currentColor"
                strokeWidth="4"
                fill="none"
              />
              <path
                className="opacity-75"
                fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
              />
            </svg>
            Connecting...
          </span>
        )}
        {connectionStatus == ConnectionStatus.Failed && (
          <span className="inline-flex items-center text-red-600">
            <svg
              className="h-5 w-5 mr-2"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth="2"
                d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
              />
            </svg>
            Connection Failed
          </span>
        )}
        {connectionStatus == ConnectionStatus.Connected && (
          <>
            <button
              className="px-6 py-2 rounded-md bg-red-50 text-red-700 border-2 border-red-200 hover:bg-red-100 transition-colors duration-200 font-medium shadow-sm"
              onClick={leaveQueue}
            >
              Exit
            </button>
          </>
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
