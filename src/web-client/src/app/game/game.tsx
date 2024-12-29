"use client";
import { useEffect } from "react";
import useWebSocket, { ConnectionStatus } from "../hooks/socket";
import { GameRequest, GameResponse } from "./requests";
import { Move } from "./model";

// Component
export type GameComponentProps = {
  serverAddress: string;
};

export default function Game({ serverAddress }: GameComponentProps) {
  const socket = useWebSocket<GameRequest, GameResponse>();
  useEffect(() => {
    const onOpenRequestProvider: () => GameRequest = () => ({
      type: "JoinGame",
    });
    if (socket.connectionStatus == ConnectionStatus.Off) {
      socket.connect(serverAddress, onOpenRequestProvider, console.log);
    }
  }, [socket, serverAddress]);

  const handleMove = (move: Move) => {
    socket.send({ type: "Move", move });
  };

  return (
    <div className="flex gap-4 justify-center">
      game game game
      {Object.values(Move).map((move) => (
        <button
          key={move}
          onClick={() => handleMove(move)}
          className="px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 transition-colors"
        >
          {move.charAt(0).toUpperCase() + move.slice(1)}
        </button>
      ))}
    </div>
  );
}
