mod flags;
mod headless;
mod metrics;
mod runner;
mod ui;

use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, anyhow};
use oxeylyzer_core::{data::Data, generate::Oxeylyzer, weights::Config};
use oxeylyzer_resources::OxeylyzerDirs;

use crate::runner::{AlgorithmKind, Budget};

fn main() -> anyhow::Result<()> {
    let flags = flags::Bench::from_env_or_exit();

    let language = flags.language.unwrap_or_else(|| "english".to_string());
    let dirs = OxeylyzerDirs::resolve().map_err(|e| anyhow!("{e}"))?;
    let config = Config::with_loaded_weights(dirs.config_file()).map_err(|e| anyhow!("{e}"))?;
    let data = Data::load(dirs.language_data_dir().join(format!("{language}.json")))
        .map_err(|e| anyhow!("{e}"))?;
    let analyzer = Oxeylyzer::new(data, config);

    let saved = oxeylyzer_repl::repl::load_layouts(dirs.layouts_dir().join(&language))
        .map_err(|e| anyhow!("{e}"))?;
    let basis_layout = match &flags.basis {
        Some(name) => saved.get(name).with_context(|| {
            let mut known = saved.keys().cloned().collect::<Vec<_>>();
            known.sort();
            format!(
                "layout '{name}' not found for language '{language}' (available: {})",
                known.join(", ")
            )
        })?,
        None => saved
            .get("sturdy")
            .or_else(|| saved.values().next())
            .with_context(|| format!("no layouts found for language '{language}'"))?,
    };
    let basis = analyzer.fast_layout(basis_layout, &[]);

    // pins are given as characters; translate to positions on the basis
    let pins: Vec<usize> = flags
        .pins
        .iter()
        .flat_map(|s| s.chars())
        .filter_map(|c| {
            let u = analyzer.mapping.get_u(c);
            basis.keys.iter().position(|&k| k == u)
        })
        .collect();

    let kinds: Vec<AlgorithmKind> = match &flags.algorithms {
        Some(s) => s
            .split(',')
            .map(|part| {
                AlgorithmKind::parse(part.trim())
                    .ok_or_else(|| anyhow!("unknown algorithm '{part}'"))
            })
            .collect::<anyhow::Result<_>>()?,
        None => AlgorithmKind::ALL.to_vec(),
    };

    let budget = match (flags.time, flags.count) {
        (Some(secs), _) => Budget::Time(Duration::from_secs(secs)),
        (None, count) => Budget::Count(count.unwrap_or(100)),
    };

    let headless = flags.headless;
    let names: Vec<&'static str> = kinds.iter().map(|k| k.name()).collect();
    let (tx, rx) = mpsc::channel();
    let cancel = AtomicBool::new(false);

    std::thread::scope(|s| -> anyhow::Result<()> {
        let kinds = &kinds;
        let analyzer = &analyzer;
        let basis = &basis;
        let pins = &pins;
        let cancel_ref = &cancel;

        s.spawn(move || runner::run_all(kinds, analyzer, basis, pins, budget, tx, cancel_ref));

        if headless {
            headless::run(rx, names);
        } else {
            let target = match budget {
                Budget::Count(n) => Some(n),
                Budget::Time(_) => None,
            };
            let title = format!(
                "language: {language} | algorithms: {} | budget: {}",
                names.join(", "),
                match budget {
                    Budget::Count(n) => format!("{n} layouts"),
                    Budget::Time(d) => format!("{}s each", d.as_secs()),
                },
            );
            let app = ui::App::new(title, names, target);
            ui::run(rx, app, &cancel)?;
        }
        Ok(())
    })
}
