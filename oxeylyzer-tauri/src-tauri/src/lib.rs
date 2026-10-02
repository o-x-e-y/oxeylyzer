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

#[derive(Serialize)]
pub struct BackendStatusDto {
    pub ready: bool,
    pub error: Option<String>,
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
    /// The config the engine and layouts were built from.
    pub config: Mutex<Config>,
    /// Set to true to request cancellation of an in-progress generation.
    pub cancel_flag: Arc<AtomicBool>,
    /// True while a generation run is in progress; prevents overlapping runs.
    pub generating: Arc<AtomicBool>,
    /// The config.toml contents this app last wrote, so the file watcher can
    /// tell its own writes apart from external edits.
    pub last_config_write: Mutex<Option<String>>,
}

/// Managed before [`AppState`], which only exists once data is downloaded and loaded.
#[derive(Default)]
pub struct Startup {
    ready: AtomicBool,
    error: Mutex<Option<String>>,
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
    // An empty corpus divides by zero; NaN would serialize as null and break the frontend.
    let f = |v: f64| if v.is_finite() { v } else { 0.0 };
    let t = &stats.trigram_stats;
    LayoutStatsDto {
        sfb: f(stats.sfb),
        dsfb: f(stats.dsfb),
        fspeed: f(stats.fspeed),
        finger_speed: stats.finger_speed.map(f),
        finger_usage: finger_usage.map(f),
        stretches: f(stats.stretches),
        scissors: f(stats.scissors),
        lsbs: f(stats.lsbs),
        pinky_ring: f(stats.pinky_ring),
        score: normalize_score(stats.score, char_total),
        inrolls: f(t.inrolls),
        outrolls: f(t.outrolls),
        onehands: f(t.onehands),
        alternates: f(t.alternates),
        alternates_sfs: f(t.alternates_sfs),
        redirects: f(t.redirects),
        redirects_sfs: f(t.redirects_sfs),
        bad_redirects: f(t.bad_redirects),
        bad_redirects_sfs: f(t.bad_redirects_sfs),
        bad_sfbs: f(t.bad_sfbs),
        sfts: f(t.sfts),
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

/// Loads every layout matched by the config's globs, plus the managed directory
/// for the current language — that's where the app saves layouts, so saved
/// layouts are always found again whatever the globs say.
fn load_all_layouts(
    config: &Config,
    dirs: &OxeylyzerDirs,
    language: &str,
) -> HashMap<String, LoadedLayout> {
    let managed = dirs.layouts_dir().join(language).join("*.dof");
    config
        .layouts
        .iter()
        .map(|p| dirs.data_dir().join(p))
        .chain(std::iter::once(managed))
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
    let mut languages: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            name.strip_suffix(".json").map(str::to_string)
        })
        .collect();
    languages.sort();
    languages
}

fn corpus_path_for(dirs: &OxeylyzerDirs, language: &str) -> PathBuf {
    dirs.language_data_dir().join(format!("{language}.json"))
}

fn panic_message(panic: &(dyn std::any::Any + Send)) -> String {
    panic
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| panic.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown error".to_string())
}

// ─── Engine Loading ───────────────────────────────────────────────────────────

struct Loaded {
    config: Config,
    engine: Arc<Oxeylyzer>,
    layouts: HashMap<String, LoadedLayout>,
}

/// Builds the engine and layout map for `config` without touching app state,
/// so a config that doesn't load is rejected before anything is replaced.
fn load_with(dirs: &OxeylyzerDirs, config: Config) -> Result<Loaded, String> {
    let corpus = dirs.data_dir().join(&config.corpus);
    let data = Data::load(&corpus)
        .map_err(|e| format!("Failed to load corpus '{}': {e}", corpus.display()))?;
    let engine = Arc::new(Oxeylyzer::new(data, config.clone()));
    let layouts = load_all_layouts(&config, dirs, &engine.language);
    Ok(Loaded {
        config,
        engine,
        layouts,
    })
}

fn install(state: &AppState, loaded: Loaded) {
    *state.config.lock().unwrap() = loaded.config;
    *state.engine.lock().unwrap() = loaded.engine;
    *state.layouts.lock().unwrap() = loaded.layouts;
}

fn write_config(state: &AppState, config: &Config) -> Result<(), String> {
    let toml = toml::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {e}"))?;
    // Recorded before writing so the watcher can never see the new file first.
    *state.last_config_write.lock().unwrap() = Some(toml.clone());
    std::fs::write(state.dirs.config_file(), toml)
        .map_err(|e| format!("Failed to write config.toml: {e}"))
}

/// Reloads config.toml from disk. Returns `false` without reloading when the
/// file holds what this app last wrote itself.
fn reload_from_disk(state: &AppState) -> Result<bool, String> {
    let content = std::fs::read_to_string(state.dirs.config_file())
        .map_err(|e| format!("Failed to read config.toml: {e}"))?;
    if state.last_config_write.lock().unwrap().as_deref() == Some(content.as_str()) {
        return Ok(false);
    }
    let config: Config =
        toml::from_str(&content).map_err(|e| format!("Failed to parse config.toml: {e}"))?;
    install(state, load_with(&state.dirs, config)?);
    Ok(true)
}

/// The config a fresh install starts with.
fn seed_config(dirs: &OxeylyzerDirs) -> Config {
    Config {
        corpus: corpus_path_for(dirs, "english"),
        layouts: vec![dirs.layouts_dir().join("english").join("*.dof")],
        corpus_configs: dirs.corpus_configs_dir().join("**").join("*.toml"),
        ..Default::default()
    }
}

/// Loads the configured engine, falling back to defaults and then to any
/// corpus that loads, so a broken config.toml can still be fixed from the
/// Config view instead of the app refusing to start.
fn initial_load(dirs: &OxeylyzerDirs) -> (Loaded, Vec<String>) {
    let mut errors = Vec::new();
    let config = Config::with_loaded_weights(dirs.config_file()).unwrap_or_else(|e| {
        errors.push(format!(
            "Couldn't read config.toml ({e}); using the default config until it's saved from the Config view."
        ));
        seed_config(dirs)
    });

    match load_with(dirs, config.clone()) {
        Ok(loaded) => return (loaded, errors),
        Err(e) => errors.push(e),
    }

    let fallbacks = std::iter::once("english".to_string())
        .chain(list_languages_from_dir(&dirs.language_data_dir()))
        .map(|language| corpus_path_for(dirs, &language));
    for corpus in fallbacks {
        let fallback = Config {
            corpus: corpus.clone(),
            ..config.clone()
        };
        if let Ok(loaded) = load_with(dirs, fallback) {
            errors.push(format!("Loaded '{}' instead.", corpus.display()));
            return (loaded, errors);
        }
    }

    errors.push("No corpus could be loaded.".to_string());
    let engine = Arc::new(Oxeylyzer::new(Data::default(), config.clone()));
    let layouts = load_all_layouts(&config, dirs, &engine.language);
    let loaded = Loaded {
        config,
        engine,
        layouts,
    };
    (loaded, errors)
}

// ─── Tauri Commands ───────────────────────────────────────────────────────────

#[tauri::command]
fn backend_status(startup: tauri::State<'_, Startup>) -> BackendStatusDto {
    BackendStatusDto {
        ready: startup.ready.load(Ordering::SeqCst),
        error: startup.error.lock().unwrap().clone(),
    }
}

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

/// Switches the corpus and persists it to config.toml, so the choice survives
/// restarts and later config saves.
#[tauri::command(async)]
fn set_language(language: String, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = Config {
        corpus: corpus_path_for(&state.dirs, &language),
        ..state.config.lock().unwrap().clone()
    };
    let loaded = load_with(&state.dirs, config)?;
    write_config(&state, &loaded.config)?;
    install(&state, loaded);
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

/// Applies a config only after it loads, so a bad corpus path is rejected
/// instead of being written to config.toml.
#[tauri::command(async)]
fn set_config(config_dto: ConfigDto, state: tauri::State<'_, AppState>) -> Result<(), String> {
    let config = Config {
        corpus: PathBuf::from(&config_dto.corpus),
        layouts: config_dto.layouts.iter().map(PathBuf::from).collect(),
        corpus_configs: PathBuf::from(&config_dto.corpus_configs),
        trigram_precision: config_dto.trigram_precision,
        max_cores: config_dto.max_cores,
        weights: dto_to_weights(&config_dto.weights),
    };
    let loaded = load_with(&state.dirs, config)?;
    write_config(&state, &loaded.config)?;
    install(&state, loaded);
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

// ─── Startup & File Watching ─────────────────────────────────────────────────

/// Downloads data on first run and builds [`AppState`] off the main thread, so
/// the window can show download progress instead of freezing.
fn init_backend(app: tauri::AppHandle, dirs: OxeylyzerDirs) {
    let mut errors = Vec::new();

    if dirs.is_first_run() {
        use oxeylyzer_resources::DownloadProgress;
        let emitter = app.clone();
        let downloaded = dirs.ensure_data(move |p| {
            let payload = match &p {
                DownloadProgress::Connecting => serde_json::json!({"status": "connecting"}),
                DownloadProgress::Downloading {
                    bytes_done,
                    bytes_total,
                } => serde_json::json!({
                    "status": "downloading",
                    "bytesDone": bytes_done,
                    "bytesTotal": bytes_total,
                }),
                DownloadProgress::Extracting => serde_json::json!({"status": "extracting"}),
                DownloadProgress::Done => serde_json::json!({"status": "done"}),
            };
            let _ = emitter.emit("download-progress", payload);
        });
        if let Err(e) = downloaded {
            errors.push(format!("Failed to download the data files: {e}."));
        }
    }

    // ensure_config is idempotent; ensure_data already calls it on first run,
    // but call it here too so a missing config is always recovered.
    if let Err(e) = dirs.ensure_config() {
        errors.push(format!("Failed to write the default config: {e}."));
    }

    let (loaded, load_errors) = initial_load(&dirs);
    errors.extend(load_errors);

    let config_file = dirs.config_file();
    let layouts_dir = dirs.layouts_dir();
    app.manage(AppState {
        engine: Mutex::new(loaded.engine),
        layouts: Mutex::new(loaded.layouts),
        dirs,
        config: Mutex::new(loaded.config),
        cancel_flag: Arc::new(AtomicBool::new(false)),
        generating: Arc::new(AtomicBool::new(false)),
        last_config_write: Mutex::new(None),
    });
    spawn_watcher(app.clone(), config_file, layouts_dir);

    let startup = app.state::<Startup>();
    if !errors.is_empty() {
        *startup.error.lock().unwrap() = Some(errors.join(" "));
    }
    startup.ready.store(true, Ordering::SeqCst);
    let _ = app.emit("backend-ready", ());
}

/// Reloads when config.toml or layout files change on disk.
fn spawn_watcher(app: tauri::AppHandle, config_file: PathBuf, layouts_dir: PathBuf) {
    use notify::{EventKind, RecursiveMode, Watcher, recommended_watcher};

    std::thread::spawn(move || {
        let (tx, rx) = std::sync::mpsc::channel::<notify::Result<notify::Event>>();
        let mut watcher = match recommended_watcher(tx) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("File watcher init failed: {e}");
                return;
            }
        };
        // Watching the directory rather than config.toml itself keeps working
        // after editors replace the file by renaming over it.
        if let Some(config_dir) = config_file.parent() {
            let _ = watcher.watch(config_dir, RecursiveMode::NonRecursive);
        }
        let _ = watcher.watch(&layouts_dir, RecursiveMode::Recursive);

        while let Ok(first) = rx.recv() {
            // Wait for the writes to settle: the first event fires as soon as a
            // file is created or truncated, before its contents are written.
            let mut events = vec![first];
            while let Ok(event) = rx.recv_timeout(Duration::from_millis(300)) {
                events.push(event);
            }
            let paths: Vec<PathBuf> = events
                .into_iter()
                .flatten()
                .filter(|e| {
                    matches!(
                        e.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    )
                })
                .flat_map(|e| e.paths)
                .collect();

            let state = app.state::<AppState>();
            if paths.iter().any(|p| p.file_name() == config_file.file_name()) {
                match reload_from_disk(&state) {
                    Ok(true) => {
                        let _ = app.emit("config-reloaded", ());
                    }
                    Ok(false) => {}
                    Err(e) => {
                        let _ = app.emit("load-error", format!("Auto-reload failed: {e}"));
                    }
                }
            } else if paths.iter().any(|p| p.extension().is_some_and(|e| e == "dof")) {
                let config = state.config.lock().unwrap().clone();
                let language = state.engine.lock().unwrap().language.clone();
                *state.layouts.lock().unwrap() = load_all_layouts(&config, &state.dirs, &language);
                let _ = app.emit("layouts-reloaded", ());
            }
        }
    });
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
            app.manage(Startup::default());
            let handle = app.handle().clone();
            std::thread::spawn(move || init_backend(handle, dirs));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_status,
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

    /// A throwaway data root holding the core crate's english corpus and gust layout.
    fn temp_dirs(tag: &str) -> (OxeylyzerDirs, PathBuf) {
        let core = Path::new("../../oxeylyzer-core/static");
        let root = std::env::temp_dir().join(format!("oxeylyzer-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dirs = OxeylyzerDirs::with_override(root.clone());
        std::fs::create_dir_all(dirs.language_data_dir()).unwrap();
        std::fs::copy(
            core.join("language_data/english.json"),
            corpus_path_for(&dirs, "english"),
        )
        .unwrap();
        std::fs::create_dir_all(dirs.layouts_dir().join("english")).unwrap();
        std::fs::copy(
            core.join("layouts/gust.dof"),
            dirs.layouts_dir().join("english/gust.dof"),
        )
        .unwrap();
        (dirs, root)
    }

    fn app_state(dirs: OxeylyzerDirs) -> AppState {
        let (loaded, _) = initial_load(&dirs);
        AppState {
            engine: Mutex::new(loaded.engine),
            layouts: Mutex::new(loaded.layouts),
            dirs,
            config: Mutex::new(loaded.config),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            generating: Arc::new(AtomicBool::new(false)),
            last_config_write: Mutex::new(None),
        }
    }

    #[test]
    fn startup_survives_a_broken_config_and_a_missing_corpus() {
        let (dirs, root) = temp_dirs("startup");

        std::fs::write(dirs.config_file(), "this is not toml [").unwrap();
        let (loaded, errors) = initial_load(&dirs);
        assert!(errors[0].contains("Couldn't read config.toml"), "{errors:?}");
        assert!(loaded.layouts.contains_key("gust"));

        let broken = Config {
            corpus: dirs.language_data_dir().join("klingon.json"),
            ..seed_config(&dirs)
        };
        std::fs::write(dirs.config_file(), toml::to_string(&broken).unwrap()).unwrap();
        let (loaded, errors) = initial_load(&dirs);
        assert!(errors.iter().any(|e| e.contains("klingon.json")), "{errors:?}");
        assert_eq!(loaded.config.corpus, corpus_path_for(&dirs, "english"));
        assert!(loaded.engine.data.char_total > 0);

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn saved_layouts_load_even_when_the_globs_point_elsewhere() {
        let (dirs, root) = temp_dirs("globs");
        let config = Config {
            layouts: vec![dirs.layouts_dir().join("dutch").join("*.dof")],
            ..seed_config(&dirs)
        };
        let layouts = load_all_layouts(&config, &dirs, "english");
        assert_eq!(
            layouts["gust"].path,
            dirs.layouts_dir().join("english/gust.dof")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn watcher_reload_skips_the_apps_own_config_writes() {
        let (dirs, root) = temp_dirs("watch");
        let state = app_state(dirs);

        let mut config = state.config.lock().unwrap().clone();
        config.weights.sfbs = -9.0;
        write_config(&state, &config).unwrap();
        assert!(!reload_from_disk(&state).unwrap());

        config.weights.sfbs = -3.0;
        std::fs::write(state.dirs.config_file(), toml::to_string(&config).unwrap()).unwrap();
        assert!(reload_from_disk(&state).unwrap());
        assert_eq!(state.config.lock().unwrap().weights.sfbs, -3.0);

        std::fs::remove_dir_all(root).unwrap();
    }

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
