import { invoke } from "@tauri-apps/api/core";
import type { Layout, BigramEntry, NgramResult } from "./mock";

export async function listLayouts(): Promise<Layout[]> {
    return invoke("list_layouts");
}

export async function listLanguages(): Promise<string[]> {
    return invoke("list_languages");
}

export async function currentLanguage(): Promise<string> {
    return invoke("current_language");
}

export async function analyzeLayout(name: string): Promise<Layout> {
    return invoke("analyze_layout", { name });
}

export async function getBigrams(
    name: string,
    category: string,
    count: number,
): Promise<BigramEntry[]> {
    return invoke("get_bigrams", { name, category, count });
}

export async function swapKeys(name: string, swaps: string): Promise<Layout> {
    return invoke("swap_keys", { name, swaps });
}

export async function getCharFrequencies(): Promise<{ char: string; percent: number }[]> {
    return invoke("get_char_frequencies");
}

export async function setLanguage(language: string): Promise<void> {
    return invoke("set_language", { language });
}

export async function reloadConfig(): Promise<void> {
    return invoke("reload_config");
}

export async function lookupNgram(ngram: string): Promise<NgramResult> {
    return invoke("lookup_ngram", { ngram });
}

export async function loadCorpus(language: string, raw: boolean): Promise<string> {
    return invoke("load_corpus", { language, raw });
}

export async function startGenerate(
    baseLayout: string,
    count: number,
    pins: string,
): Promise<void> {
    return invoke("start_generate", { baseLayout, count, pins });
}

export async function saveGenerated(index: number, name?: string): Promise<Layout> {
    return invoke("save_generated", { index, name });
}

export async function cancelGenerate(): Promise<void> {
    return invoke("cancel_generate");
}

export async function getLayoutDetail(name: string): Promise<unknown> {
    return invoke("get_layout_detail", { name });
}

export async function saveLayoutEdit(dofJson: unknown, originalName: string): Promise<void> {
    return invoke("save_layout_edit", { dofJson, originalName });
}

export async function forkLayout(name: string, newName: string): Promise<Layout> {
    return invoke("fork_layout", { name, newName });
}

export type WeightsDto = {
    lateral_penalty: number; sfbs: number; sfs: number; stretches: number;
    pinky_ring_bigrams: number; inrolls: number; outrolls: number; onehands: number;
    alternates: number; alternates_sfs: number; redirects: number; redirects_sfs: number;
    bad_redirects: number; bad_redirects_sfs: number;
    finger_weights: { lp: number; lr: number; lm: number; li: number; lt: number; rt: number; ri: number; rm: number; rr: number; rp: number };
    max_finger_use: { penalty: number; pinky: number; ring: number; middle: number; index: number; thumb: number };
};

export type ConfigDto = {
    corpus: string;
    layouts: string[];
    corpus_configs: string;
    trigram_precision: number;
    max_cores: number;
    weights: WeightsDto;
};

export async function getConfig(): Promise<ConfigDto> {
    return invoke("get_config");
}

export async function setConfig(configDto: ConfigDto): Promise<void> {
    return invoke("set_config", { configDto });
}

export async function getDefaults(): Promise<ConfigDto> {
    return invoke("get_defaults");
}

export async function getSession(): Promise<{ view: string; language: string; lastLayout: string | null }> {
    return invoke("get_session");
}

export async function setSession(session: {
    view: string;
    language: string;
    lastLayout: string | null;
}): Promise<void> {
    return invoke("set_session", { session });
}
