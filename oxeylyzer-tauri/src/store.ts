import { createSignal } from "solid-js";
import { createStore } from "solid-js/store";
import { listLayouts, listLanguages, currentLanguage, getCharFrequencies } from "./api";
import type { Layout } from "./types";

export type HeatScheme = "original" | "playground" | "v2";
export const [heatScheme, setHeatScheme] = createSignal<HeatScheme>("playground");

/** Bumped after every store refresh, so views showing computed stats re-fetch them. */
export const [dataVersion, setDataVersion] = createSignal(0);

/** A backend problem shown in the app-wide banner. */
export const [backendError, setBackendError] = createSignal<string | null>(null);

type AppStore = {
  layouts: Layout[];
  languages: string[];
  currentLanguage: string;
  /** char → frequency percent (0–100), for heatmap coloring */
  charFrequencies: Record<string, number>;
};

export const [appStore, setAppStore] = createStore<AppStore>({
  layouts: [],
  languages: [],
  currentLanguage: "",
  charFrequencies: {},
});

/**
 * Refreshes store data in place, so mounted views keep their local state
 * (generation results, comparisons, …) across the refresh.
 */
export async function refreshStore(): Promise<void> {
  try {
    await fetchStore();
  } catch (e) {
    setBackendError(`Refreshing data failed: ${e}`);
  }
}

export async function fetchStore(): Promise<void> {
  const [layouts, languages, lang, freqList] = await Promise.all([
    listLayouts(),
    listLanguages(),
    currentLanguage(),
    getCharFrequencies(),
  ]);

  const charFrequencies: Record<string, number> = {};
  for (const { char, percent } of freqList) {
    charFrequencies[char] = percent;
  }

  setAppStore({
    layouts,
    languages,
    currentLanguage: lang,
    charFrequencies,
  });
  setDataVersion((v) => v + 1);
}
