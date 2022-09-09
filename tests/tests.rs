#[cfg(test)]
mod tests {
	use oxeylyzer::*;

    #[test]
    fn load_test_json_from_file() {
        

        
        
    }

	#[test]
	fn load_language_data() {
		use load_text::*;
        use language_data::*;

		load_default("test");

		let data = LanguageData::from_file("test")
			.expect("'test.json' in static/language_data/ was not created");
		
		assert!(data.language == "test");

		let total_c = 1.0/data.characters.iter().map(|&(_, f)| f).reduce(f64::min).unwrap();
        
        assert_eq!(data.characters.get(&'e'), Some(&(2.0/total_c)));
        assert_eq!(data.characters.get(&'\''), Some(&(1.0/total_c)));

        let total_b = 1.0/data.bigrams.iter().map(|(_, &f)| f).reduce(f64::min).unwrap();

        assert_eq!(data.bigrams.get(&['\'', '*']), Some(&(1.0/total_b)));
        assert_eq!(data.bigrams.get(&['1', ':']), None);

		let total_s = 1.0/data.skipgrams.iter().map(|(_, &f)| f).reduce(f64::min).unwrap();

		assert_eq!(data.skipgrams.get(&[';', 'd']), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&['*', 'e']), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&['t', 'e']), Some(&(1.0/total_s)));
		assert_eq!(data.skipgrams.get(&['\'', 't']), None);

        let total_t = 1.0/data.trigrams.iter().map(|(_, f)| *f).reduce(f64::min).unwrap();
	}

	#[test]
	fn get_analysis() {
        use analyze::LayoutAnalysis;

		let a = LayoutAnalysis::new("test", None);
		assert!(a.is_ok());
	}

    // #[test]
	fn caching() {
		let g_opt = generate::LayoutGeneration::new(
			"tr", 1000, None
		);
		assert!(!g_opt.is_err());

		let g = g_opt.unwrap();

		let mut l = g.generate();
		let l_score = g.analysis.score(&l, 1000);
		// let cache = g.initialize_cache(&l);
		// let swap = PosPair(10, 19);
		// let s1 = g.score_swap(&mut l, &swap, &cache);
		// let s2 = g.score_swap(&mut l, &swap, &cache);
	}
}