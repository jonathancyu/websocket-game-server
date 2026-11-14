import { ClientList } from "./clientlist";

export default function Home() {
  return (
    <div className="min-h-screen bg-stone-800 p-8">
      <div className="max-w-7xl mx-auto">
        <h1 className="text-3xl font-bold text-white mb-8">
          Matchmaking Tester
        </h1>
        <p className="text-stone-400 mb-6">
          Spin up clients, join the queue, and watch matches form in real time.
        </p>
        <ClientList />
      </div>
    </div>
  );
}
