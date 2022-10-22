use crate::{
    generate::LayoutGeneration,
    utility::*,
    layout::*
};

const ALGORITHM_ROUNDS: usize = 16;
const CHANCE_TO_USE_PREVIOUS_LAYOUT: f64 = 0.2;
const RUNS_BEFORE_CHANCE_INC: usize = 1;
const RUNS_BEFORE_SWAPS_INC: usize = 1;
const RUNS_BEFORE_GTB_ROUNDS_INC: usize = 4;
const GTB_ROUNDS: usize = 1;
const MAX_RUNS: usize = usize::MAX;
const CHANCE_EXPONENTIATOR: f64 = 0.9;

struct ThreadArg {
    bestk: FastLayout,
    num_rounds: usize,
    chance_to_use_previous_layout: f64,
    number_of_swaps: usize,
    num_threads: usize,
    is_finished: bool
}

impl ThreadArg {
    pub fn new(available_chars: [char; 30]) -> Self {
        ThreadArg {
            bestk: FastLayout::random(available_chars),
            num_rounds: ALGORITHM_ROUNDS,
            chance_to_use_previous_layout: CHANCE_TO_USE_PREVIOUS_LAYOUT,
            number_of_swaps: available_chars.len() / 15,
            num_threads: 0,
            is_finished: false
        }
    }
}

impl LayoutGeneration {
    pub fn optimize_dickens(&self, possible_swaps: &[PosPair]) {
		
	}
}