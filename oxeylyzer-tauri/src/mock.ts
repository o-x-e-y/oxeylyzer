export type LayoutStats = {
    sfb: number;
    dsfb: number;
    fspeed: number;
    finger_speed: number[];
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

export type Layout = {
    name: string;
    /** 30 characters, row-major: 3 rows × 10 columns, left-to-right */
    keys: string;
    board: string;
    stats: LayoutStats;
};

export type BigramEntry = {
    bigram: string;
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

export const BIGRAM_TABS: { id: BigramTab; label: string }[] = [
    { id: "sfbs", label: "SFBs" },
    { id: "fspeed", label: "Fspeed" },
    { id: "scissors", label: "Scissors" },
    { id: "lsbs", label: "LSBs" },
    { id: "pinky-ring", label: "Pinky-Ring" },
    { id: "stretches", label: "Stretches" },
];
