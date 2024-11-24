"use client";
import { Component } from "react";
import Client from "./client";

type ClientListProps = unknown;
interface ClientListState {
  clientIds: string[];
}

export class ClientList extends Component<ClientListProps, ClientListState> {
  state = { clientIds: [] };
  constructor(props: ClientListProps) {
    super(props);
  }

  addClient = () => {
    this.setState((prevState) => ({
      clientIds: [...prevState.clientIds, crypto.randomUUID()],
    }));
  };

  removeClient = () => {
    this.setState((prevState) => {
      const newState = { ...prevState };
      if (newState.clientIds.length > 0) {
        newState.clientIds.splice(0, 1);
      }
      return newState;
    });
  };

  render() {
    return (
      <div className="space-y-4">
        <div className="w-full bg-slate-100 border-b border-slate-200 p-4 shadow-sm">
          <div className="max-w-7xl mx-auto flex items-center justify-between">
            <div className="flex items-center">
              <div className="w-32 mr-4 flex items-center">
                <span className="font-semibold text-slate-800">Clients:</span>
                <span className="ml-2 bg-slate-200 px-2 py-0.5 rounded-md font-bold text-slate-700 min-w-[2rem] text-center">
                  {this.state.clientIds.length}
                </span>
              </div>
              <div className="flex items-center space-x-2">
                <button
                  onClick={this.removeClient}
                  className="w-8 h-8 rounded-full bg-red-50 text-red-700 border-2 border-red-200 hover:bg-red-100 transition-colors duration-200 font-bold shadow-sm flex items-center justify-center"
                >
                  −
                </button>
                <button
                  onClick={this.addClient}
                  className="w-8 h-8 rounded-full bg-green-50 text-green-700 border-2 border-green-200 hover:bg-green-100 transition-colors duration-200 font-bold shadow-sm flex items-center justify-center"
                >
                  +
                </button>
              </div>
            </div>
          </div>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6 px-4">
          {this.state.clientIds.map((id, index) => (
            <div
              key={index}
              className="bg-slate-200 rounded-lg shadow-md p-4 hover:shadow-lg transition-shadow"
            >
              <h2 className="text-lg font-semibold mb-2 text-gray-700">
                Client {index}
              </h2>
              <Client id={id} />
            </div>
          ))}
        </div>
      </div>
    );
  }
}
