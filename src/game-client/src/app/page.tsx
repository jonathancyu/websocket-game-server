import { ClientList } from "./clientlist";

export default function Home() {
  return (
    <div className="min-h-screen bg-gray-100 p-8">
      <div className="max-w-7xl mx-auto">
        <h1 className="text-3xl font-bold text-gray-800 mb-8">
          Matchmaking Tester
        </h1>
        <ClientList/>
      </div>
    </div>
  );
}
