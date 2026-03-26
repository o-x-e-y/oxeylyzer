# oxeylyzer-core

`oxeylyzer-core` is the high-performance layout evaluation and generation engine behind the [Oxeylyzer](https://github.com/o-x-e-y/oxeylyzer) keyboard layout analyzer. It provides data structures and very performant cached algorithms for generating and scoring keyboard layouts.

## Features

- Heavy Focus on Performance: Uses integer-based character mappings and an extensively cached engine (`FastLayout`, `LayoutCache`) to evaluate about 12 million layout permutations every second on my Ryzen 7 thinkpad.
- Configurable Weights: Customizable penalties and rewards for different metrics.
- Corpus Processing: contains tools (`CorpusCleaner`) to clean out corpora and extract character, bigram, trigram and skip1, skip2 and skip3gram frequencies,
- Parallel Generation:  under the hood to scale layout optimization across all available CPU cores.

## Example Usage

Here is a basic example of how to initialize the core engine, load a layout, and compute its score.

```rust
use oxeylyzer_core::{
    generate::Oxeylyzer,
    weights::Config,
    data::Data,
    layout::Layout,
};

// 1. Load the optimization configuration and weights.
// This defines penalties for stretches, SFBs, max finger usage, etc.
let config = Config::with_defaults();

// 2. Load language frequency data from a processed corpus.
// (Assuming you have generated an "english.json" corpus previously)
let data = Data::load("static/language_data/english.json").unwrap();

// 3. Initialize the generator engine
let generator = Oxeylyzer::new(data, config);

// 4. Load a base layout from a `.dof` (libdof) format file.
let base_layout = Layout::load("static/layouts/qwerty.dof").unwrap();

// 5. Convert the standard layout into a FastLayout for cached generation and analysis.
let mut fast_layout = generator.fast_layout(&base_layout, &[]);

// 6. Score the layout!
let score = generator.score(&fast_layout);
println!("Score for QWERTY: {}", score);

// 7. Get detailed layout statistics (SFBs, stretches, rolls, etc.)
let stats = generator.get_layout_stats(&fast_layout);
println!("Same-finger bigrams (SFBs): {:.2}%", stats.sfb);
println!("Inrolls: {:.2}%", stats.trigram_stats.inrolls);

// 8. Generate a highly optimized layout based on the input layout
let optimized_layout = generator.generate(&fast_layout);
println!("Optimized layout:\n{}", optimized_layout.formatted_string());
```

## Creating a new Corpus

You can use the `CorpusCleaner` to process raw text into an optimized `Data` structure containing bigram and trigram frequencies.

```rust
use oxeylyzer_core::corpus_cleaner::CorpusCleaner;
use oxeylyzer_core::data::Data;

let raw_text = "This is a large text corpus that will be analyzed.";

let cleaner = CorpusCleaner::builder()
    .with_chars("abcdefghijklmnopqrstuvwxyz., ")
    .build()
    .unwrap();

// Clean the text and generate a Data object containing character/bigram/trigram frequencies
let mut data = cleaner.clean_corpus(raw_text.chars()).flatten().collect::<Data>();
```

