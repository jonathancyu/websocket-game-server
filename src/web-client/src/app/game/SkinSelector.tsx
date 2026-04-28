"use client";
import React from "react";
import { Move } from "./model";
import { Skin, SKINS } from "./skins";

type Props = {
  selected: Skin;
  onChange: (skin: Skin) => void;
};

export default function SkinSelector({ selected, onChange }: Props) {
  return (
    <div className="flex flex-wrap gap-2 items-center mb-4">
      <span className="text-sm text-gray-400 mr-1">Skin:</span>
      {SKINS.map((skin) => {
        const isActive = selected.id === skin.id;
        const icons = Object.values(Move).map((m) => skin.moves[m].icon).join(" ");
        return (
          <button
            key={skin.id}
            onClick={() => onChange(skin)}
            title={icons}
            className={`px-3 py-1.5 rounded text-sm transition-colors flex items-center gap-1.5 ${
              isActive
                ? "bg-purple-600 text-white ring-2 ring-purple-400"
                : "bg-stone-600 text-gray-300 hover:bg-stone-500"
            }`}
          >
            <span>{icons}</span>
            <span className={isActive ? "font-semibold" : ""}>{skin.name}</span>
          </button>
        );
      })}
    </div>
  );
}
