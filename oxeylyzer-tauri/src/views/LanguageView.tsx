import { createSignal, For, Show } from "solid-js";
import { MOCK_LANGUAGES, MOCK_NGRAM_RESULTS, type NgramResult } from "../mock";

export default function LanguageView() {
    const [language, setLanguage] = createSignal("english");
    const [pendingLanguage, setPendingLanguage] = createSignal("english");

    // Load corpus
    const [loadLang, setLoadLang] = createSignal("");
    const [rawFlag, setRawFlag] = createSignal(false);
    const [allFlag, setAllFlag] = createSignal(false);
    const [loadMsg, setLoadMsg] = createSignal<string | null>(null);

    // Ngram lookup
    const [ngramInput, setNgramInput] = createSignal("");
    const [ngramResult, setNgramResult] = createSignal<NgramResult | null>(null);
    const [ngramError, setNgramError] = createSignal("");

    // Reload
    const [reloadMsg, setReloadMsg] = createSignal<string | null>(null);

    function handleSetLanguage() {
        if (MOCK_LANGUAGES.includes(pendingLanguage())) {
            setLanguage(pendingLanguage());
        }
    }

    function handleLoad() {
        const lang = loadLang().trim();
        if (!lang) return;
        setLoadMsg(
            `Loaded corpus for "${lang}"${rawFlag() ? " (--raw)" : ""}${allFlag() ? " (--all)" : ""}.`,
        );
    }

    function handleNgramLookup() {
        const ng = ngramInput().trim();
        setNgramError("");
        setNgramResult(null);

        if (ng.length === 0) return;

        if (ng.length > 3) {
            setNgramError(`Ngram length ${ng.length} is not supported (max 3).`);
            return;
        }

        // Return mock result if we have one, otherwise a plausible fallback
        const known = MOCK_NGRAM_RESULTS[ng];
        if (known) {
            setNgramResult(known);
        } else if (ng.length === 1) {
            setNgramResult({ kind: "unigram", char: ng, percent: 2.345 });
        } else if (ng.length === 2) {
            const rev = ng.split("").reverse().join("");
            setNgramResult({
                kind: "bigram",
                bigram: ng,
                rev,
                total: 1.234,
                fwd: 0.987,
                bwd: 0.247,
                skipTotal: 0.543,
                skipFwd: 0.412,
                skipBwd: 0.131,
            });
        } else {
            setNgramResult({ kind: "trigram", trigram: ng, percent: 0.456 });
        }
    }

    function handleReload() {
        setReloadMsg("Config reloaded. Generated layouts retained.");
    }

    return (
        <div class="flex-1 min-h-0 overflow-y-auto flex flex-col gap-6 max-w-2xl">
            <h1 class="text-lg font-mono text-neutral-300">Language</h1>

            {/* ── Current language ─────────────────────────────── */}
            <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">
                    Current Language
                </div>

                <div class="flex items-center gap-3">
                    <span class="font-mono text-neutral-100 text-base">{language()}</span>
                </div>

                <div class="flex gap-2 items-center">
                    <select
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1"
                        value={pendingLanguage()}
                        onChange={(e) => setPendingLanguage(e.currentTarget.value)}
                    >
                        <For each={MOCK_LANGUAGES}>
                            {(lang) => <option value={lang}>{lang}</option>}
                        </For>
                    </select>
                    <button
                        class="border border-neutral-500 font-mono text-sm px-3 py-1 hover:bg-neutral-700"
                        onClick={handleSetLanguage}
                    >
                        Set Language
                    </button>
                </div>
            </section>

            {/* ── Available languages ───────────────────────────── */}
            <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">
                    Available Languages
                </div>
                <div class="flex flex-col gap-1 font-mono text-sm">
                    <For each={MOCK_LANGUAGES}>
                        {(lang) => (
                            <div class="flex items-center gap-2">
                                <span
                                    class={
                                        lang === language()
                                            ? "text-neutral-100"
                                            : "text-neutral-400"
                                    }
                                >
                                    {lang}
                                </span>
                                <Show when={lang === language()}>
                                    <span class="text-xs text-neutral-500">(current)</span>
                                </Show>
                            </div>
                        )}
                    </For>
                </div>
            </section>

            {/* ── Load corpus ───────────────────────────────────── */}
            <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">Load Corpus</div>
                <div class="text-xs text-neutral-500">
                    Processes raw text in{" "}
                    <span class="font-mono">./static/text/&lt;language&gt;/</span> and generates a
                    language data file.
                </div>

                <div class="flex gap-2 items-center flex-wrap">
                    <input
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-40"
                        placeholder="language name"
                        value={loadLang()}
                        onInput={(e) => setLoadLang(e.currentTarget.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleLoad()}
                    />

                    <label class="flex items-center gap-1.5 font-mono text-sm text-neutral-300 cursor-pointer select-none">
                        <input
                            type="checkbox"
                            class="accent-neutral-400"
                            checked={rawFlag()}
                            onChange={(e) => {
                                setRawFlag(e.currentTarget.checked);
                                if (e.currentTarget.checked) setAllFlag(false);
                            }}
                        />
                        --raw
                    </label>

                    <label class="flex items-center gap-1.5 font-mono text-sm text-neutral-300 cursor-pointer select-none">
                        <input
                            type="checkbox"
                            class="accent-neutral-400"
                            checked={allFlag()}
                            onChange={(e) => {
                                setAllFlag(e.currentTarget.checked);
                                if (e.currentTarget.checked) setRawFlag(false);
                            }}
                        />
                        --all
                    </label>

                    <button
                        class="border border-neutral-500 font-mono text-sm px-3 py-1 hover:bg-neutral-700"
                        onClick={handleLoad}
                    >
                        Load
                    </button>
                </div>

                <Show when={loadMsg()}>
                    <div class="text-xs font-mono text-neutral-400 border border-neutral-700 px-2 py-1">
                        {loadMsg()}
                    </div>
                </Show>
            </section>

            {/* ── Ngram lookup ──────────────────────────────────── */}
            <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">Ngram Lookup</div>
                <div class="text-xs text-neutral-500">
                    Enter 1–3 characters. Bigrams also show skipgram frequencies.
                </div>

                <div class="flex gap-2 items-center">
                    <input
                        class="bg-neutral-800 border border-neutral-600 text-neutral-100 font-mono text-sm px-2 py-1 w-24"
                        placeholder="th"
                        maxLength={3}
                        value={ngramInput()}
                        onInput={(e) => setNgramInput(e.currentTarget.value)}
                        onKeyDown={(e) => e.key === "Enter" && handleNgramLookup()}
                    />
                    <button
                        class="border border-neutral-500 font-mono text-sm px-3 py-1 hover:bg-neutral-700"
                        onClick={handleNgramLookup}
                    >
                        Lookup
                    </button>
                </div>

                <Show when={ngramError()}>
                    <div class="text-xs font-mono text-red-400">{ngramError()}</div>
                </Show>

                <Show when={ngramResult()} keyed>
                    {(result) => (
                        <div class="border border-neutral-700 p-3 font-mono text-sm flex flex-col gap-1">
                            {result.kind === "unigram" && (
                                <div class="flex gap-3">
                                    <span class="text-neutral-400">"{result.char}"</span>
                                    <span>{result.percent.toFixed(3)}%</span>
                                </div>
                            )}

                            {result.kind === "bigram" && (
                                <>
                                    <div class="flex gap-3">
                                        <span class="text-neutral-400">
                                            "{result.bigram}" + "{result.rev}" (bigram)
                                        </span>
                                        <span>{result.total.toFixed(3)}%</span>
                                    </div>
                                    <div class="flex gap-3 pl-4 text-neutral-400">
                                        <span>"{result.bigram}"</span>
                                        <span>{result.fwd.toFixed(3)}%</span>
                                    </div>
                                    <div class="flex gap-3 pl-4 text-neutral-400">
                                        <span>"{result.rev}"</span>
                                        <span>{result.bwd.toFixed(3)}%</span>
                                    </div>
                                    <div class="flex gap-3 mt-1">
                                        <span class="text-neutral-400">
                                            "{result.bigram}" + "{result.rev}" (skipgram)
                                        </span>
                                        <span>{result.skipTotal.toFixed(3)}%</span>
                                    </div>
                                    <div class="flex gap-3 pl-4 text-neutral-400">
                                        <span>"{result.bigram}"</span>
                                        <span>{result.skipFwd.toFixed(3)}%</span>
                                    </div>
                                    <div class="flex gap-3 pl-4 text-neutral-400">
                                        <span>"{result.rev}"</span>
                                        <span>{result.skipBwd.toFixed(3)}%</span>
                                    </div>
                                </>
                            )}

                            {result.kind === "trigram" && (
                                <div class="flex gap-3">
                                    <span class="text-neutral-400">"{result.trigram}"</span>
                                    <span>{result.percent.toFixed(3)}%</span>
                                </div>
                            )}
                        </div>
                    )}
                </Show>
            </section>

            {/* ── Reload config ─────────────────────────────────── */}
            <section class="border border-neutral-700 p-4 flex flex-col gap-3">
                <div class="text-xs text-neutral-500 uppercase tracking-widest">Reload Config</div>
                <div class="text-xs text-neutral-500">
                    Refreshes weights and default settings from{" "}
                    <span class="font-mono">config.toml</span>. Previously generated layouts are
                    retained.
                </div>
                <div>
                    <button
                        class="border border-neutral-500 font-mono text-sm px-3 py-1 hover:bg-neutral-700"
                        onClick={handleReload}
                    >
                        Reload Config
                    </button>
                </div>
                <Show when={reloadMsg()}>
                    <div class="text-xs font-mono text-neutral-400 border border-neutral-700 px-2 py-1">
                        {reloadMsg()}
                    </div>
                </Show>
            </section>
        </div>
    );
}
