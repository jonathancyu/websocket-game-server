export type Skin = {
  id: string;
  label: string;
  swatchClass: string;
  textClass: string;
  borderClass: string;
  surfaceClass: string;
  buttonClass: string;
  buttonHoverClass: string;
};

export const SKINS: Skin[] = [
  {
    id: "rose",
    label: "Rose",
    swatchClass: "bg-rose-500",
    textClass: "text-rose-700",
    borderClass: "border-rose-300",
    surfaceClass: "bg-rose-50",
    buttonClass: "bg-rose-500",
    buttonHoverClass: "hover:bg-rose-600",
  },
  {
    id: "sky",
    label: "Sky",
    swatchClass: "bg-sky-500",
    textClass: "text-sky-700",
    borderClass: "border-sky-300",
    surfaceClass: "bg-sky-50",
    buttonClass: "bg-sky-500",
    buttonHoverClass: "hover:bg-sky-600",
  },
  {
    id: "emerald",
    label: "Emerald",
    swatchClass: "bg-emerald-500",
    textClass: "text-emerald-700",
    borderClass: "border-emerald-300",
    surfaceClass: "bg-emerald-50",
    buttonClass: "bg-emerald-500",
    buttonHoverClass: "hover:bg-emerald-600",
  },
  {
    id: "violet",
    label: "Violet",
    swatchClass: "bg-violet-500",
    textClass: "text-violet-700",
    borderClass: "border-violet-300",
    surfaceClass: "bg-violet-50",
    buttonClass: "bg-violet-500",
    buttonHoverClass: "hover:bg-violet-600",
  },
  {
    id: "amber",
    label: "Amber",
    swatchClass: "bg-amber-500",
    textClass: "text-amber-700",
    borderClass: "border-amber-300",
    surfaceClass: "bg-amber-50",
    buttonClass: "bg-amber-500",
    buttonHoverClass: "hover:bg-amber-600",
  },
];

export const DEFAULT_SKIN = SKINS[0];

export function getSkin(id: string): Skin {
  return SKINS.find((skin) => skin.id === id) ?? DEFAULT_SKIN;
}
