"use client";
import useWebSocket from "./hooks/socket";

export enum Move {
  Rock = "rock",
  Paper = "paper",
  Scissors = "scissors",
}

export type MakeMove = { move: Move };
export type GameRequest = MakeMove;
// Responses
export type GameResponse = MakeMove;

// Component
export type GameComponentProps = {
  serverAddress: string;
};

export default function Game({ serverAddress }: GameComponentProps) {
  const socket = useWebSocket<GameRequest, GameResponse>();
  socket.connect(serverAddress, console.log);

  const handleMove = (move: Move) => {
    socket.send({ move });
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
