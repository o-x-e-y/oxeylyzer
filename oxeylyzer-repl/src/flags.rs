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
        /// Rank all layouts for the currently specified language. A higher score is better.
        cmd rank list {}
        /// Generate a number of layouts and displays the best 10. Note: layouts may not be correct after changing language.
        cmd generate gen g {
            required count: usize
        }
        /// Improves the the given layout by pinning keys specified in the `config.toml` and reordering everything else.
        cmd improve i optimize {
            required name: String
            required count: usize
        }
        /// Saves the nth layout that was generated. Optionally, you can provide a name as `-n <name>`.
        cmd save s {
            required n: usize
            optional name: String
        }
        /// Shows the top n sfbs on a layout. 10 by default.
        cmd sfbs {
            required name: String
            optional -c, --count count: usize
        }
        /// Set a language to be used for analysis. Tries to load corpus when not present.
        cmd language l lang {
            optional language: PathBuf
        }
        /// Include layouts stored under a different language
        cmd include {
            required language: String
        }
        /// Lists all currently available languages.
        cmd languages langs {}
        /// Loads a corpus for a certain language.
        cmd load {
            required language: PathBuf
            optional -a, --all
            optional -r, --raw
        }
        /// Gives information about a certain ngram. for 2 letter ones, skipgram info will be provided as well.
        cmd ngram n occ freq {
            required ngram: String
        }
        /// Refreshes the config, default characters for the analyzer. Will retain previously generated layouts.
        cmd reload r {}
        /// Quits the analyzer.
        cmd quit q exit {}
    }
}
