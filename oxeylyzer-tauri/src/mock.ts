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

const base: LayoutStats = {
    sfb: 0.783,
    dsfb: 6.234,
    fspeed: 3.456,
    // LP LR LM LI LT  RT RI RM RR RP
    finger_speed: [0.31, 0.42, 0.67, 1.02, 0.08, 0.09, 0.98, 0.71, 0.38, 0.29],
    stretches: 0.234,
    scissors: 0.512,
    lsbs: 0.156,
    pinky_ring: 1.234,
    score: 3.456,
    inrolls: 24.5,
    outrolls: 19.3,
    onehands: 2.1,
    alternates: 38.2,
    alternates_sfs: 7.3,
    redirects: 5.4,
    redirects_sfs: 1.2,
    bad_redirects: 1.8,
    bad_redirects_sfs: 0.4,
    bad_sfbs: 0.123,
    sfts: 0.045,
};

// Row 0: f l h v z ' w u o y
// Row 1: s r n t k c d e a i
// Row 2: x j b m q p g , . /
const SEMIMAK_JQ = "flhvz'wuoysrntkcdeaixjbmqpg,./";

// Row 0: q w f p b  j l u y ;
// Row 1: a r s t g  m n e i o
// Row 2: z x c d v  k h , . /
const COLEMAK_DH = "qwfpbjluy;arstgmneiozxcdvkh,./";

// Row 0: v m l c p  x f o u j
// Row 1: s t r d g  b h e i a
// Row 2: n z k , q  . ; / w y
const STURDY = "vmlcpxfoujstrdgbheianzk,q.;/wy";

// Row 0: w c l d k  j y r , ;
// Row 1: a r s t g  m n e i o
// Row 2: b x f z v  u h / . '
const CANARY = "wcldkjyr,;arstgmneiobxfzvuh/.'";

// Row 0: b y o u ;  l d w v z
// Row 1: c i e a ,  h t s n r
// Row 2: g ' . = /  k m p f q
const NERPS = "byou;ldwvzciea,htsnrg'.=/kmpfq";

// Row 0: f r s t m  p n i o u
// Row 1: w h e a d  . l y k '
// Row 2: q z c v g  j b , x ;
const APTMAK = "frstmpniouwhead.lyk'qzcvgjb,x;";

// Actual valid 30-char strings (double-checked below)
const layouts: Layout[] = [
    {
        name: "semimak-jq",
        keys: SEMIMAK_JQ,
        stats: { ...base, score: 3.456, sfb: 0.783, inrolls: 24.5, outrolls: 19.3 },
    },
    {
        name: "colemak-dh",
        keys: COLEMAK_DH,
        stats: { ...base, score: 3.221, sfb: 1.023, inrolls: 22.1, outrolls: 21.4 },
    },
    {
        name: "sturdy",
        keys: STURDY,
        stats: { ...base, score: 3.198, sfb: 0.912, inrolls: 23.8, outrolls: 20.0 },
    },
    {
        name: "canary",
        keys: CANARY,
        stats: { ...base, score: 3.178, sfb: 0.941, inrolls: 22.9, outrolls: 19.8 },
    },
    {
        name: "nerps",
        keys: NERPS,
        stats: { ...base, score: 3.112, sfb: 1.101, inrolls: 21.7, outrolls: 20.5 },
    },
    {
        name: "aptmak",
        keys: APTMAK,
        stats: { ...base, score: 3.089, sfb: 1.212, inrolls: 20.9, outrolls: 21.1 },
    },
];

// Sorted by score descending (best first)
export const MOCK_LAYOUTS: Layout[] = [...layouts].sort((a, b) => b.stats.score - a.stats.score);

export const MOCK_LANGUAGES: string[] = [
    "english",
    "french",
    "german",
    "spanish",
    "portuguese",
    "dutch",
];

export const MOCK_SFBS: BigramEntry[] = [
    { bigram: "sc", percent: 0.134 },
    { bigram: "ue", percent: 0.112 },
    { bigram: "nl", percent: 0.098 },
    { bigram: "eu", percent: 0.087 },
    { bigram: "pt", percent: 0.076 },
    { bigram: "oa", percent: 0.065 },
    { bigram: "ys", percent: 0.054 },
    { bigram: "ui", percent: 0.043 },
    { bigram: "rn", percent: 0.032 },
    { bigram: "lp", percent: 0.021 },
];

export const MOCK_FSPEED: BigramEntry[] = [
    { bigram: "he", percent: 0.234 },
    { bigram: "er", percent: 0.198 },
    { bigram: "th", percent: 0.176 },
    { bigram: "in", percent: 0.154 },
    { bigram: "an", percent: 0.132 },
    { bigram: "re", percent: 0.121 },
    { bigram: "on", percent: 0.109 },
    { bigram: "en", percent: 0.098 },
    { bigram: "at", percent: 0.087 },
    { bigram: "or", percent: 0.076 },
];

export const MOCK_SCISSORS: BigramEntry[] = [
    { bigram: "u,", percent: 0.045 },
    { bigram: "ex", percent: 0.032 },
    { bigram: "qs", percent: 0.021 },
    { bigram: "pl", percent: 0.018 },
    { bigram: "im", percent: 0.012 },
    { bigram: "vy", percent: 0.009 },
];

export const MOCK_LSBS: BigramEntry[] = [
    { bigram: "an", percent: 0.087 },
    { bigram: "st", percent: 0.076 },
    { bigram: "in", percent: 0.065 },
    { bigram: "re", percent: 0.054 },
    { bigram: "er", percent: 0.043 },
    { bigram: "es", percent: 0.038 },
    { bigram: "ns", percent: 0.029 },
    { bigram: "ti", percent: 0.021 },
    { bigram: "al", percent: 0.018 },
    { bigram: "le", percent: 0.012 },
];

export const MOCK_PINKYRING: BigramEntry[] = [
    { bigram: "as", percent: 0.145 },
    { bigram: "sa", percent: 0.132 },
    { bigram: "aw", percent: 0.098 },
    { bigram: "wa", percent: 0.087 },
    { bigram: "se", percent: 0.067 },
    { bigram: "es", percent: 0.058 },
    { bigram: "qw", percent: 0.043 },
    { bigram: "wq", percent: 0.034 },
    { bigram: "xs", percent: 0.021 },
    { bigram: "sx", percent: 0.017 },
];

export const MOCK_STRETCHES: BigramEntry[] = [
    { bigram: "br", percent: 0.067 },
    { bigram: "mb", percent: 0.054 },
    { bigram: "ny", percent: 0.043 },
    { bigram: "fy", percent: 0.032 },
    { bigram: "vy", percent: 0.021 },
    { bigram: "bm", percent: 0.018 },
    { bigram: "yn", percent: 0.014 },
];

export type BigramTab = "sfbs" | "fspeed" | "scissors" | "lsbs" | "pinky-ring" | "stretches";

export const BIGRAM_TABS: { id: BigramTab; label: string }[] = [
    { id: "sfbs", label: "SFBs" },
    { id: "fspeed", label: "Fspeed" },
    { id: "scissors", label: "Scissors" },
    { id: "lsbs", label: "LSBs" },
    { id: "pinky-ring", label: "Pinky-Ring" },
    { id: "stretches", label: "Stretches" },
];

export function getBigramData(tab: BigramTab): BigramEntry[] {
    switch (tab) {
        case "sfbs":
            return MOCK_SFBS;
        case "fspeed":
            return MOCK_FSPEED;
        case "scissors":
            return MOCK_SCISSORS;
        case "lsbs":
            return MOCK_LSBS;
        case "pinky-ring":
            return MOCK_PINKYRING;
        case "stretches":
            return MOCK_STRETCHES;
    }
}

export const MOCK_NGRAM_RESULTS: Record<string, NgramResult> = {
    e: { kind: "unigram", char: "e", percent: 11.162 },
    th: {
        kind: "bigram",
        bigram: "th",
        rev: "ht",
        total: 3.456,
        fwd: 3.212,
        bwd: 0.244,
        skipTotal: 1.123,
        skipFwd: 0.987,
        skipBwd: 0.136,
    },
    the: { kind: "trigram", trigram: "the", percent: 2.089 },
};

export const MOCK_GENERATED_LAYOUTS: Layout[] = [
    {
        name: "generated-1",
        keys: "flhpz'wuoysrntvcdeaixjbmqkg,./",
        stats: { ...base, score: 3.501, sfb: 0.712 },
    },
    {
        name: "generated-2",
        keys: "flhvz'wuoysrntgcdeaixjbmqkp,./",
        stats: { ...base, score: 3.489, sfb: 0.734 },
    },
    {
        name: "generated-3",
        keys: "flhvb'wuoysrntzcdeaixjmkqpg,./",
        stats: { ...base, score: 3.478, sfb: 0.751 },
    },
    {
        name: "generated-4",
        keys: "flhkz'wuoysrntvcdeaixjbmqpg,./",
        stats: { ...base, score: 3.467, sfb: 0.768 },
    },
    {
        name: "generated-5",
        keys: "flhvz'wuoysrntbcdeaikjxmqpg,./",
        stats: { ...base, score: 3.459, sfb: 0.779 },
    },
];
