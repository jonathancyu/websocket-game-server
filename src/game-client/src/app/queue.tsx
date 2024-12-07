import { Dispatch, SetStateAction, useState } from "react";
import useWebSocket, { ConnectionStatus } from "./hooks/socket";
import { match } from "ts-pattern";
import { MatchmakingRequest } from "./shared/requests";
import { MatchmakingResponse } from "./shared/responses";

type QueueProps = {
  setMessages: Dispatch<SetStateAction<MatchmakingResponse[]>>;
  joinGame: (serverAddress: string) => void;
};

enum QueueState {
  InQueue,
  Connecting,
  NotInQueue,
}

export default function Queue({ setMessages, joinGame }: QueueProps) {
  const queue = useWebSocket<MatchmakingRequest, MatchmakingResponse>();
  const [queueState, setQueueState] = useState(QueueState.NotInQueue);
  function onmessage(message: MatchmakingResponse) {
    console.log(message);
    match(message)
      .with({ type: "JoinedQueue" }, ({}) => {
        setQueueState(QueueState.InQueue);
      })
      .with({ type: "MatchFound" }, ({ server_address }) => {
        setQueueState(QueueState.NotInQueue);
        console.log("Found address " + server_address);
        joinGame(server_address);
      })
      .otherwise((msg) => {
        console.log(msg);
      });
    setMessages((previous) => [...previous, message]);
  }

  function joinQueue() {
    setQueueState(QueueState.Connecting);
    queue.connect("ws://localhost:3001", onmessage);
  }

  function leaveQueue() {
    console.log(queue);
    if (queue) {
      queue.close();
    }
  }

  function spinner() {
    return (
      <span className="inline-flex items-center text-yellow-400">
        <svg className="animate-spin h-5 w-5 mr-2" viewBox="0 0 24 24">
          <circle
            className="opacity-25"
            cx="12"
            cy="12"
            r="10"
            stroke="currentColor"
            strokeWidth="4"
            fill="none"
          />
          <path
            className="opacity-75"
            fill="currentColor"
            d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
          />
        </svg>
        Connecting...
      </span>
    );
  }
  function timer() {
    return <>InQueue</>;
  }

  return (
    <div className="space-x-4">
      {queueState == QueueState.NotInQueue && (
        <button
          className="px-6 py-2 rounded-md bg-blue-50 text-black border-2 border-blue-200 hover:bg-blue-100 transition-colors duration-200 font-medium shadow-sm"
          onClick={joinQueue}
        >
          Join Queue
        </button>
      )}
      {queueState == QueueState.Connecting && spinner()}
      {queue.connectionStatus == ConnectionStatus.Failed && (
        <span className="inline-flex items-center text-red-600">
          <svg
            className="h-5 w-5 mr-2"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth="2"
              d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
            />
          </svg>
          Connection Failed
        </span>
      )}
      {queueState == QueueState.InQueue && (
        <div className="flex flex-col items-center gap-4">
          {timer()}
          <button
            className="px-6 py-2 rounded-md bg-red-50 text-red-700 border-2 border-red-200 hover:bg-red-100 transition-colors duration-200 font-medium shadow-sm"
            onClick={leaveQueue}
          >
            Leave Queue
          </button>
        </div>
      )}
    </div>
  );
}
