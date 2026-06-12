export type LayoutStats = {
  sfb: number;
  dsfb: number;
  fspeed: number;
  finger_speed: number[];
  /** per-finger usage as % of all keystrokes, LP..RP order */
  finger_usage: number[];
  stretches: number;
  scissors: number;
  lsbs: number;
  pinky_ring: number;
  score: number;
  inrolls: number;
  outrolls: number;
  onehands: number;
  alternates: number;
  alternates_sfs: number;
  redirects: number;
  redirects_sfs: number;
  bad_redirects: number;
  bad_redirects_sfs: number;
  bad_sfbs: number;
  sfts: number;
};

/** [x, y, width, height] physical key geometry from the backend */
export type PhysKey = [number, number, number, number];

export type Layout = {
  name: string;
  /** 30 characters, row-major: 3 rows × 10 columns, left-to-right */
  keys: string;
  board: string;
  fingering_name?: string;
  stats: LayoutStats;
  /** Physical key positions — flat array, same order as `keys` */
  keyboard: PhysKey[];
  /** Keys per row */
  shape: number[];
};

export type BigramEntry = {
  bigram: string;
  percent: number;
};

export type TrigramEntry = {
  trigram: string;
  percent: number;
};

export type NgramResult =
  | { kind: "unigram"; char: string; percent: number }
  | {
      kind: "bigram";
      bigram: string;
      rev: string;
      total: number;
      fwd: number;
      bwd: number;
      skipTotal: number;
      skipFwd: number;
      skipBwd: number;
    }
  | { kind: "trigram"; trigram: string; percent: number };

export type BigramTab = "sfbs" | "fspeed" | "scissors" | "lsbs" | "pinky-ring" | "stretches";

export type TrigramTab = "inrolls" | "outrolls" | "onehands" | "alternates" | "redirects" | "sfts";

export type NgramTab = { id: BigramTab; kind: "bigram"; label: string } | { id: TrigramTab; kind: "trigram"; label: string };

export const NGRAM_TABS: NgramTab[] = [
  { id: "sfbs", kind: "bigram", label: "SFBs" },
  { id: "fspeed", kind: "bigram", label: "Fspeed" },
  { id: "scissors", kind: "bigram", label: "Scissors" },
  { id: "lsbs", kind: "bigram", label: "LSBs" },
  { id: "pinky-ring", kind: "bigram", label: "Pinky-Ring" },
  { id: "stretches", kind: "bigram", label: "Stretches" },
  { id: "inrolls", kind: "trigram", label: "Inrolls" },
  { id: "outrolls", kind: "trigram", label: "Outrolls" },
  { id: "onehands", kind: "trigram", label: "Onehands" },
  { id: "alternates", kind: "trigram", label: "Alternates" },
  { id: "redirects", kind: "trigram", label: "Redirects" },
  { id: "sfts", kind: "trigram", label: "SFTs" },
];
