/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";
import React, { useEffect, useState } from "react";
import useWebSocket, { ConnectionStatus } from "../hooks/socket";
import { GameRequest, GameResponse } from "./requests";
import { Move, Result } from "./model";
import { match } from "ts-pattern";
import { Trophy } from "lucide-react";
import { Skin, DEFAULT_SKIN } from "./skins";
import SkinSelector from "./SkinSelector";

// Component
export type GameComponentProps = {
  userId: string;
  serverAddress: string;
  endGameAction: (matchResult: Result) => void;
};

type GameState =
  | { type: "Connecting" }
  | { type: "Connected" }
  | { type: "PendingMove" }
  | { type: "MoveSent" }
  | { type: "RoundResult"; result: Result; other_move: Move }
  | {
      type: "MatchResult";
      result: Result;
      wins: number;
      total: number;
    };

export default function Game({
  userId,
  serverAddress,
  endGameAction,
}: GameComponentProps) {
  const socket = useWebSocket<GameRequest, GameResponse>(userId);
  const [gameState, setGameState] = useState<GameState>({ type: "Connecting" });
  const [myScore, setMyScore] = useState<number>(0);
  const [opponentScore, setOpponentScore] = useState<number>(0);
  const [skin, setSkin] = useState<Skin>(DEFAULT_SKIN);

  // Create socket listener
  useEffect(() => {
    const onOpenRequestProvider: () => GameRequest = () => {
      return {
        type: "JoinGame",
      };
    };

    // Message handler
    const messageHandler = (message: GameResponse) => {
      console.log("Got " + JSON.stringify(message));
      match(message)
        .with({ type: "GameJoined" }, () => {
          setGameState({ type: "Connected" });
        })
        .with({ type: "PendingMove" }, () => {
          setGameState({ type: "PendingMove" });
        })
        .with({ type: "RoundResult" }, ({ result, other_move }) => {
          setGameState({ type: "RoundResult", result, other_move });
          if (result === Result.Win) {
            setMyScore((prev) => prev + 1);
          } else if (result === Result.Loss) {
            setOpponentScore((prev) => prev + 1);
          }
        })
        .with({ type: "MatchResult" }, ({ result, wins, total }) => {
          setGameState({ type: "MatchResult", result, wins, total });
          endGameAction(result);
        })
        .otherwise((val) => console.log("TODO: " + val));
    };
    if (socket.connectionStatus == ConnectionStatus.Off) {
      socket.connect(serverAddress, onOpenRequestProvider, messageHandler);
    }
  }, [socket, serverAddress, gameState, endGameAction]);

  const makeMove = (move: Move) => {
    socket.send({ type: "Move", value: move });
    setGameState({ type: "MoveSent" });
  };

  const GameStateView = () => {
    return match(gameState)
      .with({ type: "Connecting" }, () => (
        <div className="text-yellow-500">Connecting to game server...</div>
      ))
      .with({ type: "Connected" }, () => (
        <div className="text-green-500">Connected! Ready to play.</div>
      ))
      .with({ type: "PendingMove" }, () => (
        <div className="text-blue-500">Your turn! Make a move.</div>
      ))
      .with({ type: "MoveSent" }, () => (
        <div className="text-gray-500">Move sent, waiting for opponent...</div>
      ))
      .with({ type: "RoundResult" }, ({ result, other_move }) => (
        <div className="p-4 border rounded">
          <div className="text-xl font-bold mb-2">
            {result === Result.Win
              ? "You won this round!"
              : result === Result.Loss
                ? "You lost this round."
                : "This round was a draw."}
          </div>
          <div>Opponent played: {skin.moves[other_move].icon} {skin.moves[other_move].label}</div>
          <div className="mt-2">
            Score: You {myScore} - {opponentScore} Opponent
          </div>
        </div>
      ))
      .with({ type: "MatchResult" }, ({ result, wins, total }) => (
        <div className="p-4 border rounded bg-gray-100">
          <div className="flex items-center justify-center mb-4">
            {result === Result.Win && (
              <Trophy className="text-yellow-500 mr-2" size={24} />
            )}
          </div>
          <div className="text-2xl font-bold text-center mb-2">
            {result === Result.Win
              ? "You won the Match!"
              : result === Result.Loss
                ? "You lost The Match"
                : "The match Ended In A Draw"}
          </div>
          <div className="text-center">
            Final Score: {wins} / {total}
          </div>
        </div>
      ))
      .otherwise((state) => (
        <div className="font-bold">{JSON.stringify(state)}</div>
      ));
  };

  return (
    <div className="flex-col gap-4 justify-center">
      <SkinSelector selected={skin} onChange={setSkin} />
      <GameStateView />
      <div className="flex gap-2 mt-4">
        {(gameState.type === "Connected" ||
          gameState.type === "PendingMove" ||
          gameState.type === "RoundResult") &&
          Object.values(Move).map((move) => {
            const { label, icon } = skin.moves[move];
            return (
              <button
                key={move}
                onClick={() => makeMove(move)}
                className="px-4 py-3 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors flex flex-col items-center gap-1"
              >
                <span className="text-2xl">{icon}</span>
                <span className="text-sm">{label}</span>
              </button>
            );
          })}
      </div>
    </div>
  );
}
