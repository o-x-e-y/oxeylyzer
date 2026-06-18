xflags::xflags! {
    /// Benchmark oxeylyzer search algorithms against each other.
    cmd bench {
        /// Language corpus to use (default: english)
        optional -l, --language language: String
        /// Basis layout name; defines the key set and geometry (default: sturdy, or the first found)
        optional -b, --basis basis: String
        /// Number of layouts to generate per algorithm (default: 100)
        optional -n, --count count: usize
        /// Wall-clock budget per algorithm in seconds (overrides --count)
        optional -t, --time time: u64
        /// Comma-separated algorithms: hill,sa,ils,lahc,d2,vnd (default: all)
        optional -a, --algorithms algorithms: String
        /// Characters to pin in place on the basis layout
        optional -p, --pins pins: String
        /// Print a markdown table instead of the TUI
        optional --headless
    }
}
