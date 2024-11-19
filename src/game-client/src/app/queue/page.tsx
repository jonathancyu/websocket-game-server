'use client';

import { useEffect, useState } from "react";

const useWebSocket = (url: string) => {
    const [messages, setMessages] = useState<string[]>([]);
    const [ws, setWs] = useState<WebSocket | null>(null);

    useEffect(() => {
        const socket = new WebSocket(url);
        setWs(socket);

        socket.onmessage = (event) => {
            setMessages((previous) => [...previous, event.data]);
        }

        return () => {
            socket.close();
        }
    }, [url])

    const sendMessage = (message: string) => {
        if (ws) {
            ws.send(message);
        }
    }

    return { messages, sendMessage };
}

export default function Queue() {
    const { messages, sendMessage } = useWebSocket("ws://localhost:3001");

    function joinQueue() {
        sendMessage("Test");
        console.log("Sent test");
    }


    return (
        <div className="m-2">
            <button className="bg-blue-50 text-black" onClick={joinQueue}>Join queue</button>
            <div>
                {
                    messages.map((msg, index) => (
                        <div key={index}>{msg}</div>
                    ))
                }
            </div>

        </div>

    )
}
