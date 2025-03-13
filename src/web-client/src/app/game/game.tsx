/* eslint-disable @typescript-eslint/no-unused-vars */
"use client";
import React, { useEffect, useState } from "react";
import useWebSocket, { ConnectionStatus } from "../hooks/socket";
import { GameRequest, GameResponse } from "./requests";
import { Move, Result } from "./model";
import { match } from "ts-pattern";
import { Trophy } from "lucide-react";

// Component
export type GameComponentProps = {
  userId: string;
  serverAddress: string;
  endGameAction?: () => void;
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

type AnimationMap = {
  [T in GameState["type"]]?: {
    duration: number;
    component: () => React.ReactElement;
  };
};

const animations: AnimationMap = {
  RoundResult: {
    duration: 500,
    component: () => (
      <div className="fixed inset-0 flex items-center justify-center">
        <div className="animate-bounce">
          <Trophy className="w-16 h-16 text-yellow-500" />
        </div>
      </div>
    ),
  },
};

export default function Game({
  userId,
  serverAddress,
  endGameAction,
}: GameComponentProps) {
  const socket = useWebSocket<GameRequest, GameResponse>(userId);
  const [gameState, setGameState] = useState<GameState>({ type: "Connecting" });
  // State reducer
  const [stateStack, setStateStack] = useState<GameState[]>([]);
  const [animating, setAnimating] = useState(false);

  const pushState = (state: GameState) => {
    setStateStack((stateStack) => [...stateStack, state]);
  };

  useEffect(() => {
    const popEvent = () => {
      if (stateStack.length === 0) return;

      const newStack = [...stateStack];
      newStack.pop();
      setStateStack(newStack);
      setGameState(newStack[newStack.length - 1]);
    };

    const consumeNextEvent = async () => {
      if (stateStack.length > 0 && animating === false) {
        const nextState = stateStack[stateStack.length - 1];
        setGameState(nextState);
        // Check if there's an animation for entering this state
        const animation = animations[gameState.type];
        if (!animation) {
          return;
        }

        // Play animation
        setAnimating(true);
        await new Promise((resolve) => setTimeout(resolve, animation.duration));
        setAnimating(false);
      }
      popEvent();
    };
    consumeNextEvent();
  }, [gameState, stateStack, animating]);

  // Create socket listener
  useEffect(() => {
    const onOpenRequestProvider: () => GameRequest = () => {
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
          setGameState({ type: "Connected" });
        })
        .with({ type: "PendingMove" }, (msg) => {
          pushState(msg as GameState);
        })
        .with({ type: "RoundResult" }, ({ result, other_move }) => {
          pushState({
            type: "RoundResult",
            result,
            other_move,
          });
        })
        .with({ type: "MatchResult" }, ({ result, wins, total }) => {
          pushState({
            type: "MatchResult",
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
  }, [socket, serverAddress, gameState]);

  const makeMove = (move: Move) => {
    socket.send({ type: "Move", value: move });
  };

  const GameStateView = () => {
    return match(gameState).otherwise((state) => (
      <div className="font-bold">{JSON.stringify(state)}</div>
    ));
  };

  const CurrentAnimation = gameState && animations[gameState.type]?.component;
  return (
    <div className="flex-col gap-4 justify-center">
      <GameStateView />
      {animating && CurrentAnimation && <CurrentAnimation />}
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
