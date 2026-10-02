use std::{
    collections::HashMap,
    panic::AssertUnwindSafe,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use oxeylyzer_core::{
    SPACE_CHAR,
    data::Data,
    fast_layout::{BigramPair, FastLayout},
    generate::{LayoutStats, Oxeylyzer},
    layout::{Layout, LayoutMetadata, PosPair},
    rayon::{
        self, ThreadPoolBuilder,
        iter::{IndexedParallelIterator, IntoParallelRefIterator, ParallelIterator},
    },
    weights::{Config, FingerWeights, MaxFingerUse, Weights},
};
use oxeylyzer_resources::OxeylyzerDirs;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};

// ─── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Serialize, Clone)]
pub struct LayoutStatsDto {
    pub sfb: f64,
    pub dsfb: f64,
    pub fspeed: f64,
    pub finger_speed: [f64; 10],
    /// Per-finger usage as % of all keystrokes (LP..RP order, matching finger_speed).
    pub finger_usage: [f64; 10],
    pub stretches: f64,
    pub scissors: f64,
    pub lsbs: f64,
    pub pinky_ring: f64,
    pub score: f64,
    // trigram fields, flattened to match frontend LayoutStats type
    pub inrolls: f64,
    pub outrolls: f64,
    pub onehands: f64,
    pub alternates: f64,
    pub alternates_sfs: f64,
    pub redirects: f64,
    pub redirects_sfs: f64,
    pub bad_redirects: f64,
    pub bad_redirects_sfs: f64,
    pub bad_sfbs: f64,
    pub sfts: f64,
}

#[derive(Serialize, Clone)]
pub struct LayoutDto {
    pub name: String,
    pub keys: String,
    pub board: String,
    pub fingering_name: Option<String>,
    pub stats: LayoutStatsDto,
    /// Physical key geometry: [x, y, width, height] per key (flat, same order as keys)
    pub keyboard: Vec<[f64; 4]>,
    /// Number of keys per row
    pub shape: Vec<usize>,
}

#[derive(Serialize, Clone)]
pub struct BigramEntryDto {
    pub bigram: String,
    pub percent: f64,
}

#[derive(Serialize, Clone)]
pub struct TrigramEntryDto {
    pub trigram: String,
    pub percent: f64,
}

#[derive(Serialize, Clone)]
pub struct CharFreqDto {
    pub char: String,
    pub percent: f64,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum NgramResultDto {
    Unigram {
        #[serde(rename = "char")]
        ch: String,
        percent: f64,
    },
    Bigram {
        bigram: String,
        rev: String,
        total: f64,
        fwd: f64,
        bwd: f64,
        #[serde(rename = "skipTotal")]
        skip_total: f64,
        #[serde(rename = "skipFwd")]
        skip_fwd: f64,
        #[serde(rename = "skipBwd")]
        skip_bwd: f64,
    },
    Trigram {
        trigram: String,
        percent: f64,
    },
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SessionDto {
    pub view: String,
    // Option fields default to None when missing, which keeps old session
    // files (snake_case last_layout) loadable — that key is simply ignored.
    pub last_layout: Option<String>,
    #[serde(default)]
    pub heat_scheme: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct MaxFingerUseDto {
    pub penalty: f64,
    pub pinky: f64,
    pub ring: f64,
    pub middle: f64,
    pub index: f64,
    pub thumb: f64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeightsDto {
    pub lateral_penalty: f64,
    pub sfbs: f64,
    pub sfs: f64,
    pub stretches: f64,
    pub pinky_ring_bigrams: f64,
    pub inrolls: f64,
    pub outrolls: f64,
    pub onehands: f64,
    pub alternates: f64,
    pub alternates_sfs: f64,
    pub redirects: f64,
    pub redirects_sfs: f64,
    pub bad_redirects: f64,
    pub bad_redirects_sfs: f64,
    pub finger_weights: FingerWeights,
    pub max_finger_use: MaxFingerUseDto,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ConfigDto {
    pub corpus: String,
    pub layouts: Vec<String>,
    pub corpus_configs: String,
    pub trigram_precision: usize,
    pub max_cores: usize,
    pub weights: WeightsDto,
}

// ─── App State ────────────────────────────────────────────────────────────────

/// A layout together with the file it was loaded from, so edits and deletes
/// act on that file instead of a path guessed from the layout's name.
pub struct LoadedLayout {
    pub layout: Layout,
    pub path: PathBuf,
}

pub struct AppState {
    /// The active analyzer engine, wrapped in Arc so it can be cheaply cloned for
    /// background generation without holding the lock.
    pub engine: Mutex<Arc<Oxeylyzer>>,
    /// All loaded layouts, keyed by lowercase name.
    pub layouts: Mutex<HashMap<String, LoadedLayout>>,
    /// Managed resource paths (XDG/AppData config dir in release, override in dev).
    pub dirs: OxeylyzerDirs,
    /// Cached config for reload and language switching.
    pub config: Mutex<Config>,
    /// Set to true to request cancellation of an in-progress generation.
    pub cancel_flag: Arc<AtomicBool>,
    /// True while a generation run is in progress; prevents overlapping runs.
    pub generating: Arc<AtomicBool>,
}

/// Clears the generating flag when dropped, including when generation panics.
struct GeneratingGuard(Arc<AtomicBool>);

impl Drop for GeneratingGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn normalize_score(raw: i64, char_total: i64) -> f64 {
    if char_total == 0 {
        return 0.0;
    }
    (raw as f64) / (char_total as f64) / 100.0
}

/// Per-finger usage as % of all keystrokes, in LP..RP order.
fn finger_usage_pct(engine: &Oxeylyzer, fl: &FastLayout) -> [f64; 10] {
    let total = engine.data.char_total as f64;
    let mut usage = [0.0f64; 10];
    if total == 0.0 {
        return usage;
    }
    for (i, &u) in fl.keys.iter().enumerate() {
        let finger = fl.fingers[i] as usize;
        usage[finger] += engine.data.chars()[u as usize] as f64;
    }
    usage.map(|v| v / total * 100.0)
}

fn stats_to_dto(stats: &LayoutStats, char_total: i64, finger_usage: [f64; 10]) -> LayoutStatsDto {
    let t = &stats.trigram_stats;
    LayoutStatsDto {
        sfb: stats.sfb,
        dsfb: stats.dsfb,
        fspeed: stats.fspeed,
        finger_speed: stats.finger_speed,
        finger_usage,
        stretches: stats.stretches,
        scissors: stats.scissors,
        lsbs: stats.lsbs,
        pinky_ring: stats.pinky_ring,
        score: normalize_score(stats.score, char_total),
        inrolls: t.inrolls,
        outrolls: t.outrolls,
        onehands: t.onehands,
        alternates: t.alternates,
        alternates_sfs: t.alternates_sfs,
        redirects: t.redirects,
        redirects_sfs: t.redirects_sfs,
        bad_redirects: t.bad_redirects,
        bad_redirects_sfs: t.bad_redirects_sfs,
        bad_sfbs: t.bad_sfbs,
        sfts: t.sfts,
    }
}

fn fast_layout_to_dto(
    engine: &Oxeylyzer,
    fl: &FastLayout,
    name: String,
    keys: String,
    board: String,
) -> LayoutDto {
    let stats = engine.get_layout_stats(fl);
    LayoutDto {
        name,
        keys,
        board,
        fingering_name: fl.metadata.fingering_name.as_ref().map(|n| n.to_string()),
        stats: stats_to_dto(&stats, engine.data.char_total, finger_usage_pct(engine, fl)),
        keyboard: fl
            .keyboard
            .iter()
            .map(|k| [k.x(), k.y(), k.width(), k.height()])
            .collect(),
        shape: fl.shape.inner().to_vec(),
    }
}

fn layout_to_dto(engine: &Oxeylyzer, layout: &Layout) -> LayoutDto {
    let fast = engine.fast_layout(layout, &[]);
    let keys = fast.layout_str();
    fast_layout_to_dto(engine, &fast, layout.name.clone(), keys, board_name(layout))
}

/// The named board (ortho, ansi, …), or "custom" for layouts with explicit key geometry.
fn board_name(layout: &Layout) -> String {
    match serde_json::to_value(layout)
        .ok()
        .and_then(|v| v.get("board").cloned())
    {
        Some(serde_json::Value::String(s)) => s,
        _ => "custom".to_string(),
    }
}

fn load_all_layouts(config: &Config, base_path: &Path) -> HashMap<String, LoadedLayout> {
    config
        .layouts
        .iter()
        .map(|p| base_path.join(p))
        .flat_map(|pattern| {
            glob::glob(&pattern.to_string_lossy())
                .into_iter()
                .flatten()
                .flatten()
        })
        .filter_map(|path| match Layout::load(&path) {
            Ok(layout) => Some((layout.name.to_lowercase(), LoadedLayout { layout, path })),
            Err(e) => {
                eprintln!("Error loading layout '{}': {e}", path.display());
                None
            }
        })
        .collect()
}

fn get_layout(state: &AppState, name: &str) -> Result<Layout, String> {
    state
        .layouts
        .lock()
        .unwrap()
        .get(&name.to_lowercase())
        .map(|l| l.layout.clone())
        .ok_or_else(|| format!("Layout '{name}' not found"))
}

/// Turns a layout or preset name into a file stem without path separators or
/// other characters filesystems reject.
fn file_stem(name: &str) -> String {
    name.trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_whitespace() || c.is_control() => '_',
            c => c,
        })
        .collect()
}

/// Path for a new layout file in the managed directory, refusing names that are
/// empty or already taken (on disk or by any loaded layout).
fn new_layout_path(
    dirs: &OxeylyzerDirs,
    layouts: &HashMap<String, LoadedLayout>,
    language: &str,
    name: &str,
) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("A name is required.".to_string());
    }
    let dir = dirs.layouts_dir().join(language);
    let path = dir.join(format!("{}.dof", file_stem(name).to_lowercase()));
    if layouts.contains_key(&name.to_lowercase()) || path.exists() {
        return Err(format!("A layout named '{name}' already exists."));
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Writes `json` to `path` and loads it back as the layout stored under its name.
fn write_layout(
    layouts: &mut HashMap<String, LoadedLayout>,
    path: PathBuf,
    json: &str,
) -> Result<Layout, String> {
    std::fs::write(&path, json).map_err(|e| format!("Write failed: {e}"))?;
    let layout = Layout::load(&path).map_err(|e| e.to_string())?;
    layouts.insert(
        layout.name.to_lowercase(),
        LoadedLayout {
            layout: layout.clone(),
            path,
        },
    );
    Ok(layout)
}

/// Builds a [`FastLayout`] from a base layout with an optional custom key
/// arrangement and disabled positions applied, rebuilding `char_to_finger`
/// so trigram classification matches the final arrangement.
fn custom_fast_layout(
    engine: &Oxeylyzer,
    layout: &Layout,
    keys: Option<&str>,
    disabled_indices: &[usize],
) -> Result<FastLayout, String> {
    let mut fl = engine.fast_layout(layout, &[]);

    if let Some(keys) = keys {
        let chars: Vec<char> = keys.chars().collect();
        if chars.len() != fl.keys.len() {
            return Err(format!(
                "Key count mismatch: layout has {} keys, got {}",
                fl.keys.len(),
                chars.len()
            ));
        }
        for (i, &c) in chars.iter().enumerate() {
            fl.keys[i] = engine.mapping.get_u(c);
        }
    }

    for &idx in disabled_indices {
        if idx < fl.keys.len() {
            fl.keys[idx] = 0;
        }
    }

    fl.char_to_finger.iter_mut().for_each(|f| *f = None);
    fl.keys.iter().enumerate().for_each(|(i, &c)| {
        if c != 0 {
            fl.char_to_finger[c as usize] = Some(fl.fingers[i]);
        }
    });

    Ok(fl)
}

fn pin_positions(fast: &FastLayout, engine: &Oxeylyzer, pins: &str) -> Vec<usize> {
    let pin_set: std::collections::HashSet<char> = pins.chars().collect();
    fast.keys
        .iter()
        .map(|&u| engine.mapping.get_c(u))
        .enumerate()
        .filter_map(|(i, c)| pin_set.contains(&c).then_some(i))
        .collect()
}

fn bigram_str(engine: &Oxeylyzer, fl: &FastLayout, pair: &BigramPair) -> Option<String> {
    let u1 = fl.char(pair.pair.0)?;
    let u2 = fl.char(pair.pair.1)?;
    Some(engine.mapping.map_us(&[u1, u2]).collect())
}

fn list_languages_from_dir(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".json") {
                Some(name.trim_end_matches(".json").to_string())
            } else {
                None
            }
        })
        .collect()
}

fn corpus_path_for(language_data_dir: &Path, language: &str) -> PathBuf {
    language_data_dir.join(language).with_extension("json")
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".to_string())
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command(async)]
fn list_layouts(state: tauri::State<'_, AppState>) -> Result<Vec<LayoutDto>, String> {
    let engine = state.engine.lock().unwrap().clone();
    let layouts: Vec<Layout> = state
        .layouts
        .lock()
        .unwrap()
        .values()
        .map(|l| l.layout.clone())
        .collect();
    let mut dtos: Vec<LayoutDto> = layouts
        .par_iter()
        .map(|l| layout_to_dto(&engine, l))
        .collect();
    dtos.sort_by(|a, b| b.stats.score.total_cmp(&a.stats.score));
    Ok(dtos)
}

#[tauri::command]
fn list_languages(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    Ok(list_languages_from_dir(&state.dirs.language_data_dir()))
}

#[tauri::command]
fn current_language(state: tauri::State<'_, AppState>) -> Result<String, String> {
    Ok(state.engine.lock().unwrap().language.clone())
}

#[tauri::command]
fn text_dir(state: tauri::State<'_, AppState>) -> String {
    state.dirs.text_dir().display().to_string()
}

#[tauri::command(async)]
fn analyze_layout(name: String, state: tauri::State<'_, AppState>) -> Result<LayoutDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let layout = get_layout(&state, &name)?;
    Ok(layout_to_dto(&engine, &layout))
}

#[tauri::command(async)]
fn get_bigrams(
    name: String,
    category: String,
    count: usize,
    keys: Option<String>,
    disabled_indices: Option<Vec<usize>>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<BigramEntryDto>, String> {
    let engine = state.engine.lock().unwrap().clone();
    let layout = get_layout(&state, &name)?;
    let fl = custom_fast_layout(
        &engine,
        &layout,
        keys.as_deref(),
        &disabled_indices.unwrap_or_default(),
    )?;
    let bigram_total = engine.data.bigram_total as f64;

    let frequency_entries = |pairs: Vec<BigramPair>| -> Vec<BigramEntryDto> {
        pairs
            .iter()
            .filter_map(|pair| {
                let bigram = bigram_str(&engine, &fl, pair)?;
                let raw = engine.pair_sfb(&fl, pair);
                Some(BigramEntryDto {
                    bigram,
                    percent: (raw as f64 * 100.0) / bigram_total,
                })
            })
            .collect()
    };
    let unit_pairs = |pairs: &[PosPair]| -> Vec<BigramPair> {
        pairs
            .iter()
            .map(|&pair| BigramPair { pair, dist: 1 })
            .collect()
    };

    // fspeed and stretch entries use the same scaling as the matching stat in
    // `get_layout_stats`, so the list adds up to the headline number.
    let mut entries: Vec<BigramEntryDto> = match category.as_str() {
        "sfbs" => frequency_entries(fl.fspeed_indices.all.to_vec()),
        "scissors" => frequency_entries(unit_pairs(&fl.scissor_indices.pairs)),
        "lsbs" => frequency_entries(unit_pairs(&fl.lsb_indices.pairs)),
        "pinky-ring" => frequency_entries(unit_pairs(&fl.pinky_ring_indices.pairs)),
        "fspeed" => fl
            .fspeed_indices
            .all
            .iter()
            .filter_map(|pair| {
                let bigram = bigram_str(&engine, &fl, pair)?;
                let raw = engine.pair_fspeed(&fl, pair).abs();
                Some(BigramEntryDto {
                    bigram,
                    percent: raw as f64 / bigram_total / 10.0,
                })
            })
            .collect(),
        "stretches" => fl
            .stretch_indices
            .all_pairs
            .iter()
            .filter_map(|pair| {
                let bigram = bigram_str(&engine, &fl, pair)?;
                let raw = engine.pair_stretch(&fl, pair).abs();
                Some(BigramEntryDto {
                    bigram,
                    percent: raw as f64 / bigram_total * 10.0,
                })
            })
            .collect(),
        other => return Err(format!("Unknown bigram category: '{other}'")),
    };

    entries.sort_by(|a, b| b.percent.total_cmp(&a.percent));
    entries.truncate(count);
    Ok(entries)
}

#[tauri::command(async)]
fn get_trigrams(
    name: String,
    category: String,
    count: usize,
    keys: Option<String>,
    disabled_indices: Option<Vec<usize>>,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<TrigramEntryDto>, String> {
    use oxeylyzer_core::trigram_patterns::TrigramPattern;

    let engine = state.engine.lock().unwrap().clone();
    let layout = get_layout(&state, &name)?;
    let fl = custom_fast_layout(
        &engine,
        &layout,
        keys.as_deref(),
        &disabled_indices.unwrap_or_default(),
    )?;

    let wanted: &[TrigramPattern] = match category.as_str() {
        "inrolls" => &[TrigramPattern::Inroll],
        "outrolls" => &[TrigramPattern::Outroll],
        "onehands" => &[TrigramPattern::Onehand],
        "alternates" => &[TrigramPattern::Alternate, TrigramPattern::AlternateSfs],
        "redirects" => &[
            TrigramPattern::Redirect,
            TrigramPattern::RedirectSfs,
            TrigramPattern::BadRedirect,
            TrigramPattern::BadRedirectSfs,
        ],
        "sfts" => &[TrigramPattern::Sft],
        other => return Err(format!("Unknown trigram category: '{other}'")),
    };

    let trigram_total = engine.data.trigram_total as f64;
    // gen_trigrams is sorted by frequency descending, so the first `count`
    // matches are already the most frequent ones.
    let entries: Vec<TrigramEntryDto> = engine
        .data
        .gen_trigrams()
        .iter()
        .filter(|(t, _)| wanted.contains(&engine.get_trigram_pattern(&fl, t)))
        .take(count)
        .map(|&(t, freq)| TrigramEntryDto {
            trigram: engine.mapping.map_us(&t).collect(),
            percent: freq as f64 / trigram_total * 100.0,
        })
        .collect();

    Ok(entries)
}

/// Analyze an arbitrary key arrangement (swaps + disabled keys) derived from a named base layout.
/// `keys` is the full current arrangement; `disabled_indices` are zeroed out before scoring.
/// Returns `keys` unchanged so the frontend always has the clean arrangement available.
#[tauri::command(async)]
fn analyze_custom(
    name: String,
    keys: String,
    disabled_indices: Vec<usize>,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let layout = get_layout(&state, &name)?;
    let fl = custom_fast_layout(&engine, &layout, Some(&keys), &disabled_indices)?;
    Ok(fast_layout_to_dto(
        &engine,
        &fl,
        format!("{}*", layout.name),
        keys,
        board_name(&layout),
    ))
}

#[tauri::command]
fn get_char_frequencies(state: tauri::State<'_, AppState>) -> Result<Vec<CharFreqDto>, String> {
    let engine = state.engine.lock().unwrap().clone();
    let total = engine.data.char_total as f64;
    if total == 0.0 {
        return Ok(vec![]);
    }
    let freqs: Vec<CharFreqDto> = (0..engine.data.len())
        .filter_map(|i| {
            let c = engine.mapping.get_c(i as u8);
            // Skip the three special chars at indices 0-2
            if c == char::REPLACEMENT_CHARACTER
                || c == oxeylyzer_core::SHIFT_CHAR
                || c == oxeylyzer_core::SPACE_CHAR
            {
                return None;
            }
            let count = engine.data.chars()[i] as f64;
            Some(CharFreqDto {
                char: c.to_string(),
                percent: count / total * 100.0,
            })
        })
        .collect();
    Ok(freqs)
}

#[tauri::command(async)]
fn set_language(language: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let corpus_path = corpus_path_for(&state.dirs.language_data_dir(), &language);
    let data = Data::load(&corpus_path)
        .map_err(|e| format!("Failed to load corpus for '{language}': {e}"))?;
    let new_engine = Arc::new(Oxeylyzer::new(data, config.clone()));
    let new_layouts = load_all_layouts(&config, state.dirs.data_dir());

    *state.engine.lock().unwrap() = new_engine;
    *state.layouts.lock().unwrap() = new_layouts;
    Ok(())
}

#[tauri::command(async)]
fn lookup_ngram(
    ngram: String,
    state: tauri::State<'_, AppState>,
) -> Result<NgramResultDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let data = &engine.data;

    // The corpus stores spaces as SPACE_CHAR.
    let ngram: String = ngram
        .chars()
        .map(|c| if c == ' ' { SPACE_CHAR } else { c })
        .collect();

    // get_u maps unknown characters to byte 0 (the replacement character);
    // report those instead of silently returning the replacement's stats.
    for c in ngram.chars() {
        if data.mapping.get_u(c) == 0 {
            return Err(format!("'{c}' does not appear in the current corpus."));
        }
    }

    match ngram.chars().count() {
        1 => {
            let c = ngram.chars().next().unwrap();
            let u = data.mapping.get_u(c);
            let percent = (data.get_char_u(u) as f64 / data.char_total as f64) * 100.0;
            Ok(NgramResultDto::Unigram {
                ch: c.to_string(),
                percent,
            })
        }
        2 => {
            let chars: Vec<char> = ngram.chars().collect();
            let (c1, c2) = (chars[0], chars[1]);
            let u1 = data.mapping.get_u(c1);
            let u2 = data.mapping.get_u(c2);
            let rev: String = [c2, c1].iter().collect();

            let fwd = (data.get_bigram_u([u1, u2]) as f64 / data.bigram_total as f64) * 100.0;
            let bwd = (data.get_bigram_u([u2, u1]) as f64 / data.bigram_total as f64) * 100.0;
            let skip_fwd =
                (data.get_skipgram_u([u1, u2]) as f64 / data.skipgram_total as f64) * 100.0;
            let skip_bwd =
                (data.get_skipgram_u([u2, u1]) as f64 / data.skipgram_total as f64) * 100.0;

            Ok(NgramResultDto::Bigram {
                bigram: ngram,
                rev,
                total: fwd + bwd,
                fwd,
                bwd,
                skip_total: skip_fwd + skip_bwd,
                skip_fwd,
                skip_bwd,
            })
        }
        3 => {
            let chars: Vec<char> = ngram.chars().collect();
            let t = [
                data.mapping.get_u(chars[0]),
                data.mapping.get_u(chars[1]),
                data.mapping.get_u(chars[2]),
            ];
            let &(_, occ) = data
                .gen_trigrams()
                .iter()
                .find(|&&(tf, _)| tf == t)
                .unwrap_or(&(t, 0));
            let percent = (occ as f64) / (data.trigram_total as f64) * 100.0;
            Ok(NgramResultDto::Trigram {
                trigram: ngram,
                percent,
            })
        }
        n => Err(format!(
            "Invalid ngram length {n}. Allowed: 1, 2, or 3 characters."
        )),
    }
}

#[tauri::command]
async fn load_corpus(
    language: String,
    raw: bool,
    state: tauri::State<'_, AppState>,
) -> Result<String, String> {
    use oxeylyzer_core::corpus_cleaner::CorpusCleaner;

    let source_dir = state.dirs.text_dir().join(&language);
    let out_dir = state.dirs.language_data_dir();
    if !source_dir.is_dir() {
        return Err(format!(
            "Source directory '{}' not found.",
            source_dir.display()
        ));
    }

    // Corpus processing is CPU-heavy and can take a long time for large texts —
    // run it on a blocking thread so the UI stays responsive.
    tauri::async_runtime::spawn_blocking(move || {
        let paths: Vec<PathBuf> = std::fs::read_dir(&source_dir)
            .map_err(|e| e.to_string())?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .collect();

        let cleaner = if raw {
            CorpusCleaner::raw()
        } else {
            CorpusCleaner::default()
        };

        let data = Data::from_paths(&paths, &language, &cleaner)
            .map_err(|e| format!("Failed to process corpus: {e}"))?;

        data.save(&out_dir)
            .map_err(|e| format!("Failed to save corpus: {e}"))?;

        Ok(format!(
            "Corpus '{language}' processed and saved successfully."
        ))
    })
    .await
    .map_err(|e| format!("Corpus task failed: {e}"))?
}

/// Generates one batch of layouts with the selected search algorithm.
/// Defaults follow the empirically tuned parameters from the bench suite
/// (see oxeylyzer-bench): ils (5, 25), sa (0.9997, 50k), lahc (1000, 100k).
fn generate_batch(
    engine: &Oxeylyzer,
    algorithm: &str,
    n: usize,
    basis: &FastLayout,
    pins: &[usize],
) -> Vec<FastLayout> {
    use oxeylyzer_core::generate::{
        annealing::SimulatedAnnealing, engine::Engine, hill_climber::CachedHillClimber,
        ils::IteratedLocalSearch, lahc::LateAcceptanceHillClimbing,
    };

    match algorithm {
        "ils" => IteratedLocalSearch {
            analyzer: engine,
            perturb_swaps: 5,
            rounds: 25,
        }
        .generate_n_with_pins_iter(n, basis, pins)
        .collect(),
        "sa" => SimulatedAnnealing {
            analyzer: engine,
            cooling: 0.9997,
            iters: 50_000,
        }
        .generate_n_with_pins_iter(n, basis, pins)
        .collect(),
        "lahc" => LateAcceptanceHillClimbing {
            analyzer: engine,
            history: 1_000,
            iters: 100_000,
        }
        .generate_n_with_pins_iter(n, basis, pins)
        .collect(),
        _ => CachedHillClimber { analyzer: engine }
            .generate_n_with_pins_iter(n, basis, pins)
            .collect(),
    }
}

struct GenerateRun {
    engine: Arc<Oxeylyzer>,
    algorithm: String,
    count: usize,
    base: FastLayout,
    pins: Vec<usize>,
    max_cores: usize,
    cancel: Arc<AtomicBool>,
}

/// Runs generation on a pool sized by `max_cores` and returns the top 50
/// results plus whether the run was cancelled.
fn run_generation(
    run: &GenerateRun,
    on_progress: &(dyn Fn(usize) + Sync),
) -> Result<(Vec<LayoutDto>, bool), String> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(run.max_cores)
        .build()
        .map_err(|e| format!("Failed to start worker threads: {e}"))?;

    Ok(pool.install(|| {
        // Generate in parallel batches, checking the cancel flag between
        // batches — this is what makes cancellation actually stop the work
        // instead of merely discarding its results.
        let batch = rayon::current_num_threads();
        let mut results: Vec<FastLayout> = Vec::with_capacity(run.count);
        let mut last_emit = std::time::Instant::now();

        while results.len() < run.count && !run.cancel.load(Ordering::Relaxed) {
            let n = batch.min(run.count - results.len());
            results.extend(generate_batch(&run.engine, &run.algorithm, n, &run.base, &run.pins));

            if last_emit.elapsed() >= Duration::from_millis(200) {
                last_emit = std::time::Instant::now();
                on_progress(results.len());
            }
        }

        let cancelled = run.cancel.load(Ordering::Relaxed);

        // Pre-compute scores once per layout, then sort by cached value.
        let mut scored: Vec<(i64, FastLayout)> = results
            .into_iter()
            .map(|fl| (run.engine.score(&fl), fl))
            .collect();
        scored.sort_unstable_by(|(s1, _), (s2, _)| s2.cmp(s1));

        let top = &scored[..50.min(scored.len())];
        let dtos = top
            .par_iter()
            .enumerate()
            .map(|(i, (_, fl))| {
                fast_layout_to_dto(
                    &run.engine,
                    fl,
                    format!("gen-{}", i + 1),
                    fl.layout_str(),
                    "generated".to_string(),
                )
            })
            .collect();
        (dtos, cancelled)
    }))
}

#[tauri::command]
async fn start_generate(
    base_layout: String,
    count: usize,
    pins: String,
    algorithm: Option<String>,
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    // Reject overlapping runs — two concurrent runs would double CPU usage
    // and interleave their progress events.
    if state
        .generating
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return Err("A generation run is already in progress.".to_string());
    }
    let guard = GeneratingGuard(state.generating.clone());
    state.cancel_flag.store(false, Ordering::Relaxed);

    let engine = state.engine.lock().unwrap().clone();
    let base = engine.fast_layout(&get_layout(&state, &base_layout)?, &[]);
    let run = GenerateRun {
        pins: pin_positions(&base, &engine, &pins),
        engine,
        algorithm: algorithm.unwrap_or_else(|| "hill".to_string()),
        count,
        base,
        max_cores: state.config.lock().unwrap().max_cores,
        cancel: state.cancel_flag.clone(),
    };

    std::thread::spawn(move || {
        let _guard = guard;
        let payload = match std::panic::catch_unwind(AssertUnwindSafe(|| {
            run_generation(&run, &|done| {
                let _ = app_handle.emit(
                    "generate-progress",
                    serde_json::json!({ "done": done, "total": run.count }),
                );
            })
        })) {
            Ok(Ok((results, cancelled))) => {
                serde_json::json!({ "results": results, "cancelled": cancelled })
            }
            Ok(Err(e)) => serde_json::json!({ "results": [], "cancelled": false, "error": e }),
            Err(panic) => serde_json::json!({
                "results": [],
                "cancelled": false,
                "error": format!("Generation failed: {}", panic_message(panic.as_ref())),
            }),
        };
        let _ = app_handle.emit("generate-done", payload);
    });

    Ok(())
}

#[tauri::command]
fn cancel_generate(state: tauri::State<'_, AppState>) {
    state.cancel_flag.store(true, Ordering::Relaxed);
}

/// Deletes a layout's file. Only files inside the managed layouts directory are
/// deleted; layouts loaded from other config globs belong to the user.
#[tauri::command(async)]
fn delete_layout(name: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut layouts = state.layouts.lock().unwrap();
    let key = name.to_lowercase();
    let path = &layouts
        .get(&key)
        .ok_or_else(|| format!("Layout '{name}' not found"))?
        .path;
    if !path.starts_with(state.dirs.layouts_dir()) {
        return Err(format!(
            "'{}' lives outside the managed layouts directory; delete it manually.",
            path.display()
        ));
    }
    std::fs::remove_file(path).map_err(|e| format!("Delete failed: {e}"))?;
    layouts.remove(&key);
    Ok(())
}

/// Saves a key arrangement (drag-swaps in Analyze, or a generated layout) as a
/// new layout derived from `base_name`.
#[tauri::command(async)]
fn save_custom_layout(
    base_name: String,
    keys: String,
    new_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<LayoutDto, String> {
    let engine = state.engine.lock().unwrap().clone();
    let base = get_layout(&state, &base_name)?;
    let mut fl = custom_fast_layout(&engine, &base, Some(&keys), &[])?;
    fl.name = Some(new_name.trim().to_string());

    // Serialize to .dof JSON, clearing provenance fields from the base layout.
    let layout: Layout = fl.into();
    let layout = Layout {
        metadata: Arc::new(LayoutMetadata {
            authors: vec![],
            year: None,
            link: None,
            ..(*layout.metadata).clone()
        }),
        ..layout
    };
    let json =
        serde_json::to_string_pretty(&layout).map_err(|e| format!("Serialization failed: {e}"))?;

    let mut layouts = state.layouts.lock().unwrap();
    let path = new_layout_path(&state.dirs, &layouts, &engine.language, &new_name)?;
    let saved = write_layout(&mut layouts, path, &json)?;
    Ok(layout_to_dto(&engine, &saved))
}

/// Returns the layout's .dof file as written, so fields the core doesn't model
/// (extra layers, magic, combos, description) survive an edit.
#[tauri::command(async)]
fn get_layout_detail(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let (path, layout) = {
        let layouts = state.layouts.lock().unwrap();
        let loaded = layouts
            .get(&name.to_lowercase())
            .ok_or_else(|| format!("Layout '{name}' not found"))?;
        (loaded.path.clone(), loaded.layout.clone())
    };
    match std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(json) => Ok(json),
        None => serde_json::to_value(&layout).map_err(|e| e.to_string()),
    }
}

/// Saves an edited .dof. Keeping the name (ignoring case) overwrites the file the
/// layout was loaded from; a different name creates a new layout instead.
#[tauri::command(async)]
fn save_layout_edit(
    dof_json: serde_json::Value,
    original_name: String,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let layout: Layout =
        serde_json::from_value(dof_json.clone()).map_err(|e| format!("Invalid layout: {e}"))?;
    let json = serde_json::to_string_pretty(&dof_json).map_err(|e| e.to_string())?;
    let language = state.engine.lock().unwrap().language.clone();

    let mut layouts = state.layouts.lock().unwrap();
    let original_key = original_name.to_lowercase();
    if layout.name.trim().to_lowercase() == original_key {
        let path = layouts
            .get(&original_key)
            .ok_or_else(|| format!("Layout '{original_name}' not found"))?
            .path
            .clone();
        write_layout(&mut layouts, path, &json)?;
    } else {
        let path = new_layout_path(&state.dirs, &layouts, &language, &layout.name)?;
        write_layout(&mut layouts, path, &json)?;
    }
    Ok(())
}

#[tauri::command]
fn get_session(state: tauri::State<'_, AppState>) -> Result<SessionDto, String> {
    let path = state.dirs.session_file();
    if !path.exists() {
        return Ok(SessionDto {
            view: "layouts".to_string(),
            last_layout: None,
            heat_scheme: None,
        });
    }
    let json = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_session(session: SessionDto, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let path = state.dirs.session_file();
    let json = serde_json::to_string_pretty(&session).map_err(|e| e.to_string())?;
    std::fs::write(&path, &json).map_err(|e| e.to_string())
}

// ─── Config Commands ──────────────────────────────────────────────────────────

fn config_to_dto(config: &Config) -> ConfigDto {
    let w = &config.weights;
    ConfigDto {
        corpus: config.corpus.to_string_lossy().into_owned(),
        layouts: config
            .layouts
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect(),
        corpus_configs: config.corpus_configs.to_string_lossy().into_owned(),
        trigram_precision: config.trigram_precision,
        max_cores: config.max_cores,
        weights: WeightsDto {
            lateral_penalty: w.lateral_penalty,
            sfbs: w.sfbs,
            sfs: w.sfs,
            stretches: w.stretches,
            pinky_ring_bigrams: w.pinky_ring_bigrams,
            inrolls: w.inrolls,
            outrolls: w.outrolls,
            onehands: w.onehands,
            alternates: w.alternates,
            alternates_sfs: w.alternates_sfs,
            redirects: w.redirects,
            redirects_sfs: w.redirects_sfs,
            bad_redirects: w.bad_redirects,
            bad_redirects_sfs: w.bad_redirects_sfs,
            finger_weights: w.finger_weights.clone(),
            max_finger_use: MaxFingerUseDto {
                penalty: w.max_finger_use.penalty,
                pinky: w.max_finger_use.pinky,
                ring: w.max_finger_use.ring,
                middle: w.max_finger_use.middle,
                index: w.max_finger_use.index,
                thumb: w.max_finger_use.thumb,
            },
        },
    }
}

fn dto_to_weights(w: &WeightsDto) -> Weights {
    Weights {
        lateral_penalty: w.lateral_penalty,
        sfbs: w.sfbs,
        sfs: w.sfs,
        stretches: w.stretches,
        pinky_ring_bigrams: w.pinky_ring_bigrams,
        inrolls: w.inrolls,
        outrolls: w.outrolls,
        onehands: w.onehands,
        alternates: w.alternates,
        alternates_sfs: w.alternates_sfs,
        redirects: w.redirects,
        redirects_sfs: w.redirects_sfs,
        bad_redirects: w.bad_redirects,
        bad_redirects_sfs: w.bad_redirects_sfs,
        finger_weights: w.finger_weights.clone(),
        max_finger_use: MaxFingerUse {
            penalty: w.max_finger_use.penalty,
            pinky: w.max_finger_use.pinky,
            ring: w.max_finger_use.ring,
            middle: w.max_finger_use.middle,
            index: w.max_finger_use.index,
            thumb: w.max_finger_use.thumb,
        },
    }
}

#[tauri::command]
fn get_config(state: tauri::State<'_, AppState>) -> Result<ConfigDto, String> {
    let config = state.config.lock().unwrap();
    Ok(config_to_dto(&config))
}

#[tauri::command(async)]
fn set_config(config_dto: ConfigDto, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let new_weights = dto_to_weights(&config_dto.weights);
    let new_config = Config {
        corpus: PathBuf::from(&config_dto.corpus),
        layouts: config_dto.layouts.iter().map(PathBuf::from).collect(),
        corpus_configs: PathBuf::from(&config_dto.corpus_configs),
        trigram_precision: config_dto.trigram_precision,
        max_cores: config_dto.max_cores,
        weights: new_weights,
    };

    let config_path = state.dirs.config_file();
    let toml = toml::to_string_pretty(&new_config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    std::fs::write(&config_path, toml).map_err(|e| format!("Write failed: {e}"))?;

    // Rebuild engine with new config
    let corpus_path = state.dirs.data_dir().join(&new_config.corpus);
    let data = Data::load(&corpus_path).map_err(|e| format!("Failed to load corpus: {e}"))?;
    let new_engine = Arc::new(Oxeylyzer::new(data, new_config.clone()));
    let new_layouts = load_all_layouts(&new_config, state.dirs.data_dir());

    *state.config.lock().unwrap() = new_config;
    *state.engine.lock().unwrap() = new_engine;
    *state.layouts.lock().unwrap() = new_layouts;
    Ok(())
}

#[tauri::command]
fn get_defaults() -> Result<ConfigDto, String> {
    Ok(config_to_dto(&Config::with_defaults()))
}

// ─── Weight Presets ───────────────────────────────────────────────────────────

fn preset_path(dirs: &OxeylyzerDirs, name: &str) -> Result<PathBuf, String> {
    let stem = file_stem(name);
    if stem.is_empty() {
        return Err("A preset name is required.".to_string());
    }
    Ok(dirs.weight_presets_dir().join(format!("{stem}.toml")))
}

#[tauri::command]
fn list_weight_presets(state: tauri::State<'_, AppState>) -> Result<Vec<String>, String> {
    let dir = state.dirs.weight_presets_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".toml").map(str::to_string)
        })
        .collect();
    names.sort();
    Ok(names)
}

#[tauri::command]
fn save_weight_preset(
    name: String,
    weights: WeightsDto,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = preset_path(&state.dirs, &name)?;
    std::fs::create_dir_all(state.dirs.weight_presets_dir()).map_err(|e| e.to_string())?;
    let toml = toml::to_string_pretty(&weights).map_err(|e| e.to_string())?;
    std::fs::write(&path, toml).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_weight_preset(
    name: String,
    state: tauri::State<'_, AppState>,
) -> Result<WeightsDto, String> {
    let path = preset_path(&state.dirs, &name)?;
    let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    toml::from_str::<WeightsDto>(&s).map_err(|e| format!("Failed to parse preset '{name}': {e}"))
}

// ─── Reload Helper ────────────────────────────────────────────────────────────

fn reload_state(state: &AppState) -> Result<(), String> {
    let config = Config::with_loaded_weights(state.dirs.config_file())
        .map_err(|e| format!("Failed to reload config: {e}"))?;
    let corpus_path = state.dirs.data_dir().join(&config.corpus);
    let data = Data::load(&corpus_path).map_err(|e| format!("Failed to load corpus: {e}"))?;
    let new_engine = Arc::new(Oxeylyzer::new(data, config.clone()));
    let new_layouts = load_all_layouts(&config, state.dirs.data_dir());
    *state.config.lock().unwrap() = config;
    *state.engine.lock().unwrap() = new_engine;
    *state.layouts.lock().unwrap() = new_layouts;
    Ok(())
}

// ─── Entry Point ──────────────────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dirs = if let Ok(p) = std::env::var("OXEYLYZER_DATA_DIR") {
                OxeylyzerDirs::with_override(PathBuf::from(p))
            } else {
                OxeylyzerDirs::resolve().expect("failed to resolve data directory")
            };

            // On first run, download data files synchronously before initialising the
            // engine — corpus and layout files must exist before we try to load them.
            // Progress events are emitted so a frontend loading screen can react.
            if dirs.is_first_run() {
                use oxeylyzer_resources::DownloadProgress;
                let app_handle = app.handle().clone();
                dirs.ensure_data(move |p| {
                    let payload = match &p {
                        DownloadProgress::Connecting => {
                            serde_json::json!({"status": "connecting"})
                        }
                        DownloadProgress::Downloading {
                            bytes_done,
                            bytes_total,
                        } => {
                            serde_json::json!({
                                "status": "downloading",
                                "bytesDone": bytes_done,
                                "bytesTotal": bytes_total,
                            })
                        }
                        DownloadProgress::Extracting => {
                            serde_json::json!({"status": "extracting"})
                        }
                        DownloadProgress::Done => serde_json::json!({"status": "done"}),
                    };
                    let _ = app_handle.emit("download-progress", payload);
                })
                .expect("failed to download resources");
            }

            // ensure_config is idempotent; ensure_data already calls it on first run,
            // but call it here too so a missing config is always recovered.
            dirs.ensure_config()
                .expect("failed to write default config");

            let config = Config::with_loaded_weights(dirs.config_file())
                .expect("failed to load config.toml");

            let corpus_path = dirs.data_dir().join(&config.corpus);
            let data = Data::load(&corpus_path).expect("failed to load corpus");

            let engine = Arc::new(Oxeylyzer::new(data, config.clone()));
            let layouts = load_all_layouts(&config, dirs.data_dir());

            let watch_config = dirs.config_file();
            let watch_layouts = dirs.layouts_dir();

            app.manage(AppState {
                engine: Mutex::new(engine),
                layouts: Mutex::new(layouts),
                dirs,
                config: Mutex::new(config),
                cancel_flag: Arc::new(AtomicBool::new(false)),
                generating: Arc::new(AtomicBool::new(false)),
            });

            // File watcher: auto-reload when config.toml or layout files change.
            {
                use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};
                let app_handle = app.handle().clone();
                std::thread::spawn(move || {
                    let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
                    let mut watcher = match recommended_watcher(tx) {
                        Ok(w) => w,
                        Err(e) => {
                            eprintln!("File watcher init failed: {e}");
                            return;
                        }
                    };
                    let _ = watcher.watch(&watch_config, RecursiveMode::NonRecursive);
                    let _ = watcher.watch(&watch_layouts, RecursiveMode::Recursive);
                    let mut last_reload = std::time::Instant::now()
                        .checked_sub(std::time::Duration::from_secs(5))
                        .unwrap_or_else(std::time::Instant::now);
                    for event in rx.into_iter().flatten() {
                        if !matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                            continue;
                        }
                        if last_reload.elapsed() < std::time::Duration::from_secs(2) {
                            continue;
                        }
                        last_reload = std::time::Instant::now();
                        let state = app_handle.state::<AppState>();

                        // Only a config.toml change requires rebuilding the engine.
                        // Layout file changes — including the app's own saves — just
                        // refresh the layout map.
                        let config_changed = event.paths.iter().any(|p| p.ends_with("config.toml"));
                        if config_changed {
                            if let Err(e) = reload_state(&state) {
                                eprintln!("Auto-reload failed: {e}");
                            } else {
                                let _ = app_handle.emit("config-reloaded", ());
                            }
                        } else {
                            let config = state.config.lock().unwrap().clone();
                            *state.layouts.lock().unwrap() =
                                load_all_layouts(&config, state.dirs.data_dir());
                            let _ = app_handle.emit("layouts-reloaded", ());
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_layouts,
            list_languages,
            current_language,
            text_dir,
            analyze_layout,
            analyze_custom,
            get_bigrams,
            get_trigrams,
            get_char_frequencies,
            set_language,
            lookup_ngram,
            load_corpus,
            start_generate,
            cancel_generate,
            get_layout_detail,
            save_layout_edit,
            delete_layout,
            save_custom_layout,
            get_session,
            set_session,
            get_config,
            set_config,
            get_defaults,
            list_weight_presets,
            save_weight_preset,
            load_weight_preset,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_stem_strips_path_separators_and_keeps_dots() {
        assert_eq!(file_stem("../etc/passwd"), ".._etc_passwd");
        assert_eq!(file_stem(" my layout v1.5 "), "my_layout_v1.5");
        assert_eq!(file_stem("a:b*c?d"), "a_b_c_d");
    }

    #[test]
    fn new_layout_path_keeps_dotted_names_and_rejects_taken_ones() {
        let root = std::env::temp_dir().join(format!("oxeylyzer-test-{}", std::process::id()));
        let dirs = OxeylyzerDirs::with_override(root.clone());
        let mut layouts = HashMap::new();

        let path = new_layout_path(&dirs, &layouts, "english", "Gen v1.5").unwrap();
        assert_eq!(path.file_name().unwrap(), "gen_v1.5.dof");
        assert!(new_layout_path(&dirs, &layouts, "english", "  ").is_err());

        std::fs::write(&path, "{}").unwrap();
        assert!(new_layout_path(&dirs, &layouts, "english", "gen v1.5").is_err());

        layouts.insert(
            "qwerty".to_string(),
            LoadedLayout {
                layout: Layout::load("../../oxeylyzer-core/static/layouts/gust.dof").unwrap(),
                path: PathBuf::from("/elsewhere/qwerty.dof"),
            },
        );
        assert!(new_layout_path(&dirs, &layouts, "english", "QWERTY").is_err());

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn config_round_trips_through_toml() {
        let config = Config {
            corpus: PathBuf::from("/data/\"quoted\" dir/english.json"),
            ..Config::with_defaults()
        };
        let toml = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml).unwrap();
        assert_eq!(parsed.corpus, config.corpus);
        assert_eq!(parsed.weights.stretches, config.weights.stretches);
        assert_eq!(parsed.layouts, config.layouts);
    }
}
