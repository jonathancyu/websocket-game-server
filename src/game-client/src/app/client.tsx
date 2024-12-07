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
  function joinGame(serverAddress: string) {
    setState({ screen: "game", serverAddress });
  }

  return (
    <div className="m-2 space-y-4 text-black min-h-[200px]">
      {state.screen == "queue" && <Queue joinGame={joinGame} />}
      {state.screen == "game" && <Game serverAddress={state.serverAddress} />}
    </div>
  );
}
