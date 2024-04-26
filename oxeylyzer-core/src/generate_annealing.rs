use nanorand::{Rng, WyRand};
use std::f64;

use ansi_rgb::{rgb, Colorable};

use crate::generate::LayoutGeneration;
use crate::language_data::LanguageData;
use crate::layout::*;

use crate::utility::*;

fn acceptance_probability(old_cost: f64, new_cost: f64, temperature: f64) -> f64 {
    // Calculate the acceptance probability based on the cost difference and temperature
    if new_cost < old_cost {
        1.0
    } else {
        ((old_cost - new_cost) / temperature).exp()
    }
}

fn simulated_annealing(
    sel: &crate::generate::LayoutGeneration,
    initial_solution: FastLayout,
    initial_temperature: f64,
    cooling_rate: f64,
    num_iterations: usize,
) -> (FastLayout, f64) {
    let mut current_solution = initial_solution.clone();
    let mut best_solution = initial_solution.matrix;
    let mut cache = sel.initialize_cache(&initial_solution);

    let mut current_cost = cache.total_score();
    let mut best_cost = current_cost;

    let mut temperature = initial_temperature;

    let mut rng = WyRand::new();

    for _ in 0..num_iterations {
        let new_swap = POSSIBLE_SWAPS[rng.generate_range(0..POSSIBLE_SWAPS.len())];

        let new_cost = sel.score_swap_cached(&mut current_solution, &new_swap, &cache); // Generate a new neighbor solution

        let acceptance_prob = acceptance_probability(current_cost, new_cost, temperature);

        if acceptance_prob > rng.generate() {
            sel.accept_swap(&mut current_solution, &new_swap, &mut cache);
            current_cost = new_cost;
        }

        if new_cost > best_cost {
            best_solution = current_solution.matrix;
            best_cost = new_cost;
        }

        temperature *= cooling_rate;
    }

    (FastLayout::from(best_solution), best_cost)
}

pub fn heatmap_heat(data: &LanguageData, c: u8) -> String {
    let complement = 215.0 - *data.characters.get(c as usize).unwrap_or(&0.0) * 1720.0;
    let complement = complement.max(0.0) as u8;
    let heat = rgb(215, complement, complement);
    let c = data.convert_u8.from_single(c);
    format!("{}", c.to_string().fg(heat))
}

pub fn heatmap_string(data: &LanguageData, layout: &FastLayout) -> String {
    let mut print_str = String::new();

    for (i, c) in layout.matrix.iter().enumerate() {
        if i % 10 == 0 && i > 0 {
            print_str.push('\n');
        }
        if (i + 5) % 10 == 0 {
            print_str.push(' ');
        }
        print_str.push_str(heatmap_heat(data, *c).as_str());
        print_str.push(' ');
    }

    print_str
}

pub fn test() {
    let gen = LayoutGeneration::new("english", "static", None).unwrap();

    let initial_solution = FastLayout::random(gen.chars_for_generation);
    let initial_temperature = 100.0;
    let cooling_rate = 0.999995;
    let num_iterations = 50_000_000;

    let (best_solution, best_cost) = simulated_annealing(
        &gen,
        initial_solution,
        initial_temperature,
        cooling_rate,
        num_iterations,
    );

    let printable = heatmap_string(&gen.data, &best_solution);

    println!("Best solution found:\n{}", printable);
    println!("Best cost found: {}", best_cost);
}
