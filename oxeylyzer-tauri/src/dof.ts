// Token-level editing of .dof layer rows. Keys in a row are whitespace-separated
// tokens, and a token can be longer than one character ("spc", "@l2", "\~").

export function rowTokens(row: string): string[] {
  return row.split(/\s+/).filter(Boolean);
}

/** Replaces one token in place, keeping the row's spacing. */
function replaceToken(row: string, idx: number, token: string): string {
  let n = -1;
  return row.replace(/\S+/g, (t) => (++n === idx ? token : t));
}

export function flatTokens(rows: string[]): string[] {
  return rows.flatMap(rowTokens);
}

/** Returns new rows with the token at flat (row-major) index `idx` replaced. */
export function setToken(rows: string[], idx: number, token: string): string[] {
  let offset = 0;
  return rows.map((row) => {
    const n = rowTokens(row).length;
    const updated = idx >= offset && idx < offset + n ? replaceToken(row, idx - offset, token) : row;
    offset += n;
    return updated;
  });
}

/** The token that types `ch`, escaping the characters .dof reserves. */
export function charToken(ch: string): string {
  if (ch === " ") return "spc";
  if (ch === "~" || ch === "*") return "\\" + ch;
  return ch;
}

const SPECIAL_LABELS: Record<string, string> = {
  spc: "␣",
  space: "␣",
  sft: "⇑",
  shft: "⇑",
  shift: "⇑",
  st: "⇑",
  rpt: "↻",
  repeat: "↻",
};

/** What a key shows on the keyboard: its character, a symbol, or the raw token. */
export function tokenLabel(token: string): string {
  if (token === "~" || token === "*") return "";
  if (token === "\\~" || token === "\\*") return token[1];
  if (Array.from(token).length === 1) return token;
  return SPECIAL_LABELS[token.toLowerCase()] ?? token;
}
