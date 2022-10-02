#[cfg(test)]
mod tests {
	use oxeylyzer::*;

	#[test]
	fn load_language_data() {
		use load_text::*;
        use language_data::*;

		load_default("test");

		let data = LanguageData::from_file("static/language_data","test")
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
	fn get_generator() {

		let a = generate::LayoutGeneration::new("test", "static", 1000, None);
		assert!(a.is_ok());
	}
}