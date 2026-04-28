"use client";
import React from "react";
import { Skin, SKINS } from "./skins";

type Props = {
  selected: Skin;
  onChange: (skin: Skin) => void;
};

export default function SkinSelector({ selected, onChange }: Props) {
  return (
    <div className="flex gap-2 items-center mb-4">
      <span className="text-sm text-gray-400">Skin:</span>
      {SKINS.map((skin) => (
        <button
          key={skin.id}
          onClick={() => onChange(skin)}
          className={`px-3 py-1 rounded text-sm transition-colors ${
            selected.id === skin.id
              ? "bg-purple-600 text-white"
              : "bg-stone-600 text-gray-300 hover:bg-stone-500"
          }`}
        >
          {skin.moves.Rock.icon} {skin.name}
        </button>
      ))}
    </div>
  );
}
