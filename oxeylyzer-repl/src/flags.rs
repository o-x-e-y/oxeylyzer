use std::path::PathBuf;

xflags::xflags! {
    cmd repl {
        /// Analyze a layout. You can also specify a number to analyze a previously generated layout.
        cmd analyze a view layout {
            required name_or_nr: String
        }
        /// Compare two layouts
        cmd compare c comp cmp {
            required name1: String
            required name2: String
        }
        /// Swaps a number of keys on a certain layout. Cycles between one key and the next:
        /// swap sturdy ab -> swaps ab
        /// swap sturdy abc -> swaps a -> b -> c
        /// swap sturdy ab abc -> swaps a -> b and then a -> b -> c
        cmd swap {
            required name: String
            repeated swaps: String
        }
        /// Rank all layouts for the currently specified language. A higher score is better.
        cmd rank list {}
        /// Improves the the given layout. Optionally, you can provide a list of pinned characters
        /// to keep in place during optimization.
        cmd generate gen g improve i optimize {
            required name: String
            optional count: usize
            /// Sets pinned characters on the layout to optimize, `-p abc` pins `abc`.
            optional -p, --pins pins: String
        }
        /// Saves a layout by index or name. Optionally, you can provide a save name as a second argument.
        cmd save s {
            required name_or_nr: String
            optional name: String
        }
        /// Removes a saved layout by name, deleting it from disk.
        cmd remove rm {
            required name: String
            /// Skip the confirmation prompt.
            optional -y, --yes
        }
        /// Shows the top n sfbs on a layout. 10 by default.
        cmd sfbs {
            required name: String
            optional -c, --count count: usize
        }
        /// Shows the top n fspeed pairs on a layout. 10 by default.
        cmd fspeed {
            required name: String
            optional -c, --count count: usize
        }
        /// Shows the top n scissors on a layout. 10 by default.
        cmd scissors {
            required name: String
            optional -c, --count count: usize
        }
        /// Shows the top n lsbs on a layout. 10 by default.
        cmd lsbs {
            required name: String
            optional -c, --count count: usize
        }
        /// Shows the top n pinky-ring bigrams on a layout. 10 by default.
        cmd pinkyring pinky-ring pr {
            required name: String
            optional -c, --count count: usize
        }
        /// Shows the top n stretches on a layout. 10 by default.
        cmd stretches {
            required name: String
            optional -c, --count count: usize
            }
        /// Set a language to be used for analysis. Tries to load corpus when not present.
        cmd language l lang {
            optional language: PathBuf
        }
        /// Include layouts stored under a different language
        cmd include {
            repeated languages: PathBuf
        }
        /// Lists all currently available languages.
        cmd languages langs {}
        /// Loads a corpus for a certain language.
        cmd load {
            /// The name of the corpus. This value will be used when looking for a corpus config,
            /// which is the configuration used to clean the corpus.
            required language: String
            /// If set, processes the corpus as-is without cleaning it.
            optional -r, --raw
            /// If set, processes all corpora found in ./static/text where the folder name is the
            /// language name.
            optional -a, --all
        }
        /// Gives information about a certain ngram. for 2 letter ones, skipgram info will be provided as well.
        cmd ngram n occ freq {
            required ngram: String
        }
        /// Show current weights, or modify them. Pass weight flags to update values (e.g. `config
        /// --sfbs 1.0`). Pass `--edit` to open the config file in $EDITOR.
        cmd config cfg weights {
            /// Open the config file in $EDITOR (or $VISUAL, vi, notepad as fallbacks).
            optional --edit
            optional --sfbs sfbs: f64
            optional --sfs sfs: f64
            optional --inrolls inrolls: f64
            optional --outrolls outrolls: f64
            optional --onehands onehands: f64
            optional --alternates alternates: f64
            optional --alternates-sfs alternates_sfs: f64
            optional --redirects redirects: f64
            optional --redirects-sfs redirects_sfs: f64
            optional --bad-redirects bad_redirects: f64
            optional --bad-redirects-sfs bad_redirects_sfs: f64
            optional --stretches stretches: f64
            optional --pinky-ring pinky_ring_bigrams: f64
            optional --lateral-penalty lateral_penalty: f64
            optional --trigram-precision trigram_precision: usize
            optional --max-cores max_cores: usize
        }
        /// Refreshes the config, default characters for the analyzer. Will retain previously generated layouts.
        cmd reload r {}
        /// Quits the analyzer.
        cmd quit q exit {}
    }
}
