/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";

import { useState } from "react";
import Game from "./game/game";
import Queue from "./queue/queue";
import { Result } from "./game/model";
import { DEFAULT_SKIN, getSkin, SKINS } from "./shared/skins";

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
  const [skinId, setSkinId] = useState(DEFAULT_SKIN.id);
  const skin = getSkin(skinId);

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
      <div className="absolute top-0 right-0 text-sm font-semibold rounded-full px-3 py-1 border bg-white/90 shadow-sm">
        Record: {record.wins}W - {record.losses}L
      </div>
      <div className={`rounded-xl border-2 ${skin.borderClass} bg-white/90 shadow-lg p-4 space-y-4`}>
        <div className="space-y-3">
          <div className="flex items-center justify-between gap-4">
            <div>
              <div className={`text-xs font-bold uppercase tracking-[0.2em] ${skin.textClass}`}>
                Player skin
              </div>
              <div className="text-sm text-slate-600">
                Pick a color theme for this client.
              </div>
            </div>
            <div className={`rounded-full px-3 py-1 text-xs font-semibold ${skin.surfaceClass} ${skin.textClass}`}>
              {skin.label}
            </div>
          </div>
          <div className="grid grid-cols-2 gap-2 sm:grid-cols-3 lg:grid-cols-5">
            {SKINS.map((option) => {
              const selected = option.id === skin.id;

              return (
                <button
                  key={option.id}
                  type="button"
                  onClick={() => setSkinId(option.id)}
                  className={`flex items-center gap-2 rounded-lg border-2 px-3 py-2 text-left transition ${
                    selected
                      ? "border-slate-900 bg-slate-50 shadow-md"
                      : "border-slate-200 bg-white hover:border-slate-400"
                  }`}
                >
                  <span className={`h-4 w-4 rounded-full ${option.swatchClass}`} />
                  <span className="text-sm font-medium">{option.label}</span>
                </button>
              );
            })}
          </div>
        </div>
        {state.screen == "queue" && (
          <Queue userId={userId} joinGame={joinGame} skin={skin} />
        )}
        {state.screen == "game" && (
          <Game
            userId={userId}
            serverAddress={state.serverAddress}
            endGameAction={endGameAction}
            skin={skin}
          />
        )}
      </div>
    </div>
  );
}
