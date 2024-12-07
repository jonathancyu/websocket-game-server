/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useState } from "react";
import { MatchmakingResponse } from "./shared/responses";
import Game from "./game";
import Queue from "./queue";

type ClientProps = {
  id: string;
};

type ClientState =
  | { screen: "queue" }
  | { screen: "game"; serverAddress: string };

export default function Client({ id }: ClientProps) {
  const [state, setState] = useState<ClientState>({ screen: "queue" });
  const [messages, setMessages] = useState<MatchmakingResponse[]>([]);

  function joinGame(serverAddress: string) {
    setState({ screen: "game", serverAddress });
  }

  return (
    <div className="m-2 space-y-4 text-black min-h-[200px]">
      {messages.length > 0 && (
        <div className="bottom-2 right-2 h-48 overflow-y-auto bg-white/50 rounded-lg shadow-sm">
          <div className="flex flex-col p-2 space-y-1">
            {messages.map((message, index) => (
              <div
                key={index}
                className="p-2 bg-black/5 backdrop-blur-sm rounded"
              >
                <pre className="whitespace-pre-wrap break-words text-xs">
                  {JSON.stringify(message, null, 0)}
                </pre>
              </div>
            ))}
          </div>
        </div>
      )}
      {state.screen == "queue" && (
        <Queue setMessages={setMessages} joinGame={joinGame} />
      )}
      {state.screen == "game" && <Game serverAddress={state.serverAddress} />}
    </div>
  );
}
