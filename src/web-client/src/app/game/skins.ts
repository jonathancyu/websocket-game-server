import { Move } from "./model";

export type Skin = {
  id: string;
  name: string;
  moves: Record<Move, { label: string; icon: string }>;
};

export const SKINS: Skin[] = [
  {
    id: "classic",
    name: "Classic",
    moves: {
      [Move.Rock]: { label: "Rock", icon: "🪨" },
      [Move.Paper]: { label: "Paper", icon: "📄" },
      [Move.Scissors]: { label: "Scissors", icon: "✂️" },
    },
  },
  {
    id: "animals",
    name: "Animals",
    moves: {
      [Move.Rock]: { label: "Turtle", icon: "🐢" },
      [Move.Paper]: { label: "Octopus", icon: "🐙" },
      [Move.Scissors]: { label: "Crab", icon: "🦀" },
    },
  },
  {
    id: "space",
    name: "Space",
    moves: {
      [Move.Rock]: { label: "Asteroid", icon: "☄️" },
      [Move.Paper]: { label: "Planet", icon: "🪐" },
      [Move.Scissors]: { label: "Rocket", icon: "🚀" },
    },
  },
  {
    id: "food",
    name: "Food",
    moves: {
      [Move.Rock]: { label: "Potato", icon: "🥔" },
      [Move.Paper]: { label: "Flatbread", icon: "🫓" },
      [Move.Scissors]: { label: "Fork", icon: "🍴" },
    },
  },
];

export const DEFAULT_SKIN = SKINS[0];
