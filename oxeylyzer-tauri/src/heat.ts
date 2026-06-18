import { heatScheme } from "./store";

export function heatColorOriginal(pct: number): string {
  const c = Math.max(0, Math.min(215, 215 - (pct / 100) * 1720));
  return `background-color:rgb(215,${Math.round(c)},${Math.round(c)});color:#111`;
}

export function heatColorPlayground(pct: number): string {
  const p = pct / 100;
  const v = p * 30 + Math.log(p * 120 + 1);
  const b = 95;
  return `background-color:rgb(${Math.round(b * 0.9 + v * 18)},${Math.round(b * 1.3 - v * 10)},${Math.round(b * 1.325 - v * 10)})`;
}

export function heatColorV2(pct: number): string {
  const f = Math.min(pct, 14) / 14;
  return `background-color:rgb(${Math.round(140 + f * 115)},${Math.round(140 - f * 140)},${Math.round(140 - f * 140)});color:#111`;
}

/** Heat style for a frequency percent using the active color scheme. */
export function heatStyleFor(pct: number): string {
  switch (heatScheme()) {
    case "original":
      return heatColorOriginal(pct);
    case "v2":
      return heatColorV2(pct);
    default:
      return heatColorPlayground(pct);
  }
}
