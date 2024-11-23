"use client"
import { Component } from "react";
import Queue from "./queue/page";

export class ClientList extends Component {
    render() {
        return <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {Array.from({ length: 10 }).map((_, index) => (
            <div
              key={index}
              className="bg-white rounded-lg shadow-md p-4 hover:shadow-lg transition-shadow"
            >
              <h2 className="text-lg font-semibold mb-2 text-gray-700">
                Client {index + 1}
              </h2>
              <Queue />
            </div>
          ))}
        </div>;
    }
}
