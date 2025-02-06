/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useState } from "react";
import Game from "./game/game";
import Queue from "./queue/queue";

type ClientProps = {
  id: string;
};

type ClientState =
  | { screen: "queue" }
  | { screen: "game"; serverAddress: string };

export default function Client({ id: userId }: ClientProps) {
  const [state, setState] = useState<ClientState>({ screen: "queue" });
  function joinGame(serverAddress: string) {
    setState({ screen: "game", serverAddress });
  }

  return (
    <div className="m-2 space-y-4 text-black min-h-[200px]">
      {state.screen == "queue" && <Queue userId={userId} joinGame={joinGame} />}
      {state.screen == "game" && (
        <Game userId={userId} serverAddress={state.serverAddress} />
      )}
    </div>
  );
}
