use crate::{
    utility::*,
    generate::{LayoutGeneration, pinned_swaps},
    layout::{FastLayout, Layout}
};

use rayon::iter::{ParallelIterator, IntoParallelIterator};

impl LayoutGeneration {
    pub fn gen_iteratively(&self) {
        let mut pins = Vec::<usize>::with_capacity(30);
        let mut best = FastLayout::random(self.chars_for_generation);
        
        for i in 0..30 {
            println!("step: {}/30, pinned: '{}'", i+1, pins.iter().map(|i| best.c(*i)).collect::<String>());
            println!("indexes pinned: {pins:?}");
            println!("updated best:\n{best}\nscore: {}\n", best.score);

            let possible_swaps = pinned_swaps(&pins);
            let layouts = (0..250)
                .into_par_iter()
                .map(|_| {
                    let l = FastLayout::random_pins(best.matrix, &pins);
                    let mut cache = self.initialize_cache(&l);
                    self.optimize(l, &mut cache, &possible_swaps)
                })
                .collect::<Vec<_>>();

            best = layouts.into_iter().max_by(|a, b|
                a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Less)
            ).unwrap();

            let new_pin = best.matrix.iter()
                .position(|&c| c == self.chars_for_generation[i])
                .unwrap();
            
            pins.push(new_pin);
        }
        println!("best:\n{best}\nscore: {}", best.score);
    }
}

#[cfg(test)]
mod tests {
    use crate::generate::LayoutGeneration;

    #[test]
    fn iterative() {
        let gen =
            LayoutGeneration::new("english", "static", None).unwrap();
        gen.gen_iteratively();
    }
}