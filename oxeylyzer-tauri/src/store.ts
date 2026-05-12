import { createStore } from "solid-js/store";
import { listLayouts, listLanguages, currentLanguage, getCharFrequencies } from "./api";
import type { Layout } from "./mock";

type AppStore = {
    layouts: Layout[];
    languages: string[];
    currentLanguage: string;
    /** char → frequency percent (0–100), for heatmap coloring */
    charFrequencies: Record<string, number>;
    loading: boolean;
    error: string | null;
};

export const [appStore, setAppStore] = createStore<AppStore>({
    layouts: [],
    languages: [],
    currentLanguage: "",
    charFrequencies: {},
    loading: false,
    error: null,
});

export async function initStore(): Promise<void> {
    setAppStore("loading", true);
    setAppStore("error", null);
    try {
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
            loading: false,
        });
    } catch (e) {
        setAppStore({ error: String(e), loading: false });
    }
}
