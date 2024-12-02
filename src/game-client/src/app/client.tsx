/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useEffect, useState } from "react";
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
    <div className="m-2 space-y-4 text-black relative min-h-[200px]">
      {messages.length > 0 && (
        <div className="absolute top-1 right-1 w-72 flex flex-col items-end">
          {messages.map((message, index) => (
            <div
              key={index}
              className="mb-1 p-1 bg-black/5 backdrop-blur-sm rounded shadow-sm max-w-full"
            >
              <pre className="whitespace-pre-wrap break-words text-xs">
                {JSON.stringify(message, null, 0)}
              </pre>
            </div>
          ))}
        </div>
      )}
      {/* {state.screen == "queue" && ( */}
      {/*   <Queue setMessages={setMessages} joinGame={joinGame} /> */}
      {/* )} */}
      {state.screen == "game" && <Game serverAddress={""} />}
    </div>
  );
}
