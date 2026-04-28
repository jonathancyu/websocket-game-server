/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useState, useEffect } from "react";
import Game from "./game/game";
import Queue from "./queue/queue";
import { Result } from "./game/model";
import { Skin, SKINS, DEFAULT_SKIN } from "./game/skins";
import SkinSelector from "./game/SkinSelector";

type ClientProps = {
  id: string;
};

type ClientState =
  | { screen: "queue" }
  | { screen: "game"; serverAddress: string };

export default function Client({ id: userId }: ClientProps) {
  const [state, setState] = useState<ClientState>({ screen: "queue" });
  const [matchResult, setMatchResult] = useState<Result | null>(null);
  const [record, setRecord] = useState({ wins: 0, losses: 0 });
  const [skin, setSkin] = useState<Skin>(DEFAULT_SKIN);

  useEffect(() => {
    const saved = localStorage.getItem("selectedSkinId");
    if (saved) {
      const found = SKINS.find((s) => s.id === saved);
      if (found) setSkin(found);
    }
  }, []);

  const handleSkinChange = (s: Skin) => {
    setSkin(s);
    localStorage.setItem("selectedSkinId", s.id);
  };

  function joinGame(serverAddress: string) {
    setState({ screen: "game", serverAddress });
  }

  const endGameAction = (result: Result) => {
    setState({ screen: "queue" });
    setMatchResult(result);

    // Update the record based on the match result
    if (result === Result.Win) {
      setRecord(prev => ({ ...prev, wins: prev.wins + 1 }));
    } else if (result === Result.Loss) {
      setRecord(prev => ({ ...prev, losses: prev.losses + 1 }));
    } else if (result === Result.Draw) {
      setRecord(prev => ({
        ...prev,
        wins: prev.wins + 0.5,
        losses: prev.losses + 0.5
      }));
    }
  }

  return (
    <div className="m-2 space-y-4 text-black min-h-[200px] relative">
      <div className="absolute top-0 right-0 text-sm font-semibold">
        Record: {record.wins}W - {record.losses}L
      </div>
      <SkinSelector selected={skin} onChange={handleSkinChange} />
      {state.screen == "queue" && <Queue userId={userId} joinGame={joinGame} />}
      {state.screen == "game" && (
        <Game userId={userId} serverAddress={state.serverAddress} skin={skin} endGameAction={endGameAction}/>
      )}
    </div>
  );
}
