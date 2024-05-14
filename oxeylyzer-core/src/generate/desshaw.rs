use crate::{
    utility::*,
    generate::LayoutGeneration,
    layout::FastLayout
};

use arrayvec::ArrayVec;
use ahash::AHashMap as HashMap;

trait LayoutHeuristic {
    fn evaluate(incomplete: IncompleteLayout);
}

#[derive(Default)]
struct IncompleteLayout {
    layout: FastLayout,
    chars_to_place: [char; 30],
    optimal_positions: [usize; 30],
    best_scores: [f64; 30],
    chars_placed: usize
}

impl IncompleteLayout {
    pub fn new(layout: FastLayout, chars_to_use: [char; 30]) -> Self {
        Self {
            layout,
            chars_to_place: chars_to_use,
            chars_placed: 0,
            ..Default::default()
        }
    }

    pub fn add_char(&mut self, i: usize) {
        let c = self.chars_to_place[self.chars_placed];
        self.chars_placed += 1;

        self.layout.matrix[i] = c;
        self.layout.char_to_finger.insert(c, I_TO_COL[i]);
    }

    pub fn remove_char(&mut self, c: char) {
        let i = *self.layout.char_to_finger.get(&c).unwrap();
        self.layout.matrix[i] = ' ';

        self.chars_placed -= 1;
    }
}

impl LayoutGeneration {
    pub fn desshaw(&self, smie: &mut IncompleteLayout) {
        for i in 0..30 {
            smie.add_char(i);
            if smie.chars_placed == 1 {
                let score = self.char_effort(&smie.layout, i);
                if score < 0.0 {}
            }
        }
    }
}