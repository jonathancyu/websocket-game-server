/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";
import { useEffect, useState } from "react";
import useWebSocket, { ConnectionStatus } from "../hooks/socket";
import { GameRequest, GameResponse } from "./requests";
import { Move, Result } from "./model";
import { match } from "ts-pattern";

// Component
export type GameComponentProps = {
  serverAddress: string;
  endGameAction: () => void;
};

type GameState =
  | { type: "Connecting" }
  | { type: "AnimatingConnected" }
  | { type: "PendingMove" }
  | { type: "MoveSent" }
  | { type: "AnimatingRoundResult"; result: Result; other_move: Move }
  | {
      type: "AnimatingMatchResult";
      result: Result;
      wins: number;
      total: number;
    };

export default function Game({
  serverAddress,
  endGameAction,
}: GameComponentProps) {
  const socket = useWebSocket<GameRequest, GameResponse>();
  const [gameState, setGameState] = useState<GameState>({ type: "Connecting" });

  // Create socket
  let count = 0;
  useEffect(() => {
    const onOpenRequestProvider: () => GameRequest = () => {
      console.log("AA -" + socket.connectionStatus + " - " + count);
      count += 1;
      return {
        type: "JoinGame",
      };
    };
    const stateStack: GameState[] = [];

    // Message handler
    const messageHandler = (message: GameResponse) => {
      console.log("Got " + JSON.stringify(message));
      match(message)
        .with({ type: "GameJoined" }, () => {
          if (gameState.type != "Connecting") {
            console.log("Warning: Got JoinGame when not connecting");
          }
          setGameState({ type: "AnimatingConnected" });
        })
        .with({ type: "PendingMove" }, (msg) => {
          stateStack.push(msg as GameState);
        })
        .with({ type: "RoundResult" }, ({ result, other_move }) => {
          stateStack.push({
            type: "AnimatingRoundResult",
            result,
            other_move,
          });
        })
        .with({ type: "MatchResult" }, ({ result, wins, total }) => {
          stateStack.push({
            type: "AnimatingMatchResult",
            result,
            wins,
            total,
          });
        })
        .otherwise((val) => console.log("TODO: " + val));
    };
    if (socket.connectionStatus == ConnectionStatus.Off) {
      socket.connect(serverAddress, onOpenRequestProvider, messageHandler);
    }
  }, [socket, serverAddress, gameState.type]);

  const makeMove = (move: Move) => {
    socket.send({ type: "Move", move });
  };

  const GameStateView = () => {
    return match(gameState).otherwise((state) => (
      <div className="font-bold">{JSON.stringify(state)}</div>
    ));
  };

  return (
    <div className="flex-col gap-4 justify-center">
      <GameStateView />
      <div>
        {Object.values(Move).map((move) => (
          <button
            key={move}
            onClick={() => makeMove(move)}
            className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
          >
            {move.charAt(0).toUpperCase() + move.slice(1)}
          </button>
        ))}
      </div>
    </div>
  );
}
