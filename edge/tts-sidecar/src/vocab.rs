//! Phoneme → token-ID vocabulary (114 entries).
//!
//! Extracted from the Kokoro-82M ONNX model's `config.json`. IDs are NOT
//! contiguous — the gaps come from the upstream tokenizer. BOS/EOS padding
//! (token 0) is added by the caller (`kokoro.rs`), not here.

use std::collections::HashMap;

/// Build the IPA + punctuation → i64 vocab map.
pub fn build_vocab() -> HashMap<char, i64> {
    let mut map = HashMap::with_capacity(114);
    map.insert(';', 1i64);
    map.insert(':', 2i64);
    map.insert(',', 3i64);
    map.insert('.', 4i64);
    map.insert('!', 5i64);
    map.insert('?', 6i64);
    map.insert('—', 9i64);
    map.insert('…', 10i64);
    map.insert('"', 11i64);
    map.insert('(', 12i64);
    map.insert(')', 13i64);
    map.insert('\u{201C}', 14i64); // "
    map.insert('\u{201D}', 15i64); // "
    map.insert(' ', 16i64);
    map.insert('\u{0303}', 17i64); // ̃
    map.insert('\u{02A3}', 18i64); // ʣ
    map.insert('\u{02A5}', 19i64); // ʥ
    map.insert('\u{02A6}', 20i64); // ʦ
    map.insert('\u{02A8}', 21i64); // ʨ
    map.insert('\u{1D5D}', 22i64); // ᵝ
    map.insert('\u{AB67}', 23i64); // ꭧ
    map.insert('A', 24i64);
    map.insert('I', 25i64);
    map.insert('O', 31i64);
    map.insert('Q', 33i64);
    map.insert('S', 35i64);
    map.insert('T', 36i64);
    map.insert('W', 39i64);
    map.insert('Y', 41i64);
    map.insert('\u{1D4A}', 42i64); // ᵊ
    map.insert('a', 43i64);
    map.insert('b', 44i64);
    map.insert('c', 45i64);
    map.insert('d', 46i64);
    map.insert('e', 47i64);
    map.insert('f', 48i64);
    map.insert('h', 50i64);
    map.insert('i', 51i64);
    map.insert('j', 52i64);
    map.insert('k', 53i64);
    map.insert('l', 54i64);
    map.insert('m', 55i64);
    map.insert('n', 56i64);
    map.insert('o', 57i64);
    map.insert('p', 58i64);
    map.insert('q', 59i64);
    map.insert('r', 60i64);
    map.insert('s', 61i64);
    map.insert('t', 62i64);
    map.insert('u', 63i64);
    map.insert('v', 64i64);
    map.insert('w', 65i64);
    map.insert('x', 66i64);
    map.insert('y', 67i64);
    map.insert('z', 68i64);
    map.insert('\u{0251}', 69i64);  // ɑ
    map.insert('\u{0250}', 70i64);  // ɐ
    map.insert('\u{0252}', 71i64);  // ɒ
    map.insert('\u{00E6}', 72i64);  // æ
    map.insert('\u{03B2}', 75i64);  // β
    map.insert('\u{0254}', 76i64);  // ɔ
    map.insert('\u{0255}', 77i64);  // ɕ
    map.insert('\u{00E7}', 78i64);  // ç
    map.insert('\u{0256}', 80i64);  // ɖ
    map.insert('\u{00F0}', 81i64);  // ð
    map.insert('\u{02A4}', 82i64);  // ʤ
    map.insert('\u{0259}', 83i64);  // ə
    map.insert('\u{025A}', 85i64);  // ɚ
    map.insert('\u{025B}', 86i64);  // ɛ
    map.insert('\u{025C}', 87i64);  // ɜ
    map.insert('\u{025F}', 90i64);  // ɟ
    map.insert('\u{0261}', 92i64);  // ɡ
    map.insert('\u{0265}', 99i64);  // ɥ
    map.insert('\u{0268}', 101i64); // ɨ
    map.insert('\u{026A}', 102i64); // ɪ
    map.insert('\u{029D}', 103i64); // ʝ
    map.insert('\u{026F}', 110i64); // ɯ
    map.insert('\u{0270}', 111i64); // ɰ
    map.insert('\u{014B}', 112i64); // ŋ
    map.insert('\u{0273}', 113i64); // ɳ
    map.insert('\u{0272}', 114i64); // ɲ
    map.insert('\u{0274}', 115i64); // ɴ
    map.insert('\u{00F8}', 116i64); // ø
    map.insert('\u{0278}', 118i64); // ɸ
    map.insert('\u{03B8}', 119i64); // θ
    map.insert('\u{0153}', 120i64); // œ
    map.insert('\u{0279}', 123i64); // ɹ
    map.insert('\u{027E}', 125i64); // ɾ
    map.insert('\u{027B}', 126i64); // ɻ
    map.insert('\u{0281}', 128i64); // ʁ
    map.insert('\u{027D}', 129i64); // ɽ
    map.insert('\u{0282}', 130i64); // ʂ
    map.insert('\u{0283}', 131i64); // ʃ
    map.insert('\u{0288}', 132i64); // ʈ
    map.insert('\u{02A7}', 133i64); // ʧ
    map.insert('\u{028A}', 135i64); // ʊ
    map.insert('\u{028B}', 136i64); // ʋ
    map.insert('\u{028C}', 138i64); // ʌ
    map.insert('\u{0263}', 139i64); // ɣ
    map.insert('\u{0264}', 140i64); // ɤ
    map.insert('\u{03C7}', 142i64); // χ
    map.insert('\u{028E}', 143i64); // ʎ
    map.insert('\u{0292}', 147i64); // ʒ
    map.insert('\u{0294}', 148i64); // ʔ
    map.insert('\u{02C8}', 156i64); // ˈ
    map.insert('\u{02CC}', 157i64); // ˌ
    map.insert('\u{02D0}', 158i64); // ː
    map.insert('\u{02B0}', 162i64); // ʰ
    map.insert('\u{02B2}', 164i64); // ʲ
    map.insert('\u{2193}', 169i64); // ↓
    map.insert('\u{2192}', 171i64); // →
    map.insert('\u{2197}', 172i64); // ↗
    map.insert('\u{2198}', 173i64); // ↘
    map.insert('\u{1D7B}', 177i64); // ᵻ
    map
}

/// Convert a phoneme string to token IDs, skipping chars not in the vocab.
///
/// Returns an error if the resulting ID list is empty. BOS/EOS padding (0) is
/// NOT applied here — the ONNX inference layer adds it.
pub fn phonemes_to_ids(phonemes: &str, vocab: &HashMap<char, i64>) -> Result<Vec<i64>, String> {
    let ids: Vec<i64> = phonemes
        .chars()
        .filter_map(|c| vocab.get(&c).copied())
        .collect();

    if ids.is_empty() {
        return Err(format!(
            "G2P produced no known phonemes for input (phoneme string: {phonemes:?})"
        ));
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_phonemes_map_to_expected_ids() {
        let vocab = build_vocab();
        // 'h' → 50, 'i' → 51 (the IPA vowel entry)
        let ids = phonemes_to_ids("hi", &vocab).unwrap();
        assert_eq!(ids, vec![50, 51]);
    }

    #[test]
    fn unknown_chars_are_skipped() {
        let vocab = build_vocab();
        // 'h' (50) is in vocab, '€' is not, 'i' (51) is in vocab
        let ids = phonemes_to_ids("h€i", &vocab).unwrap();
        assert_eq!(ids, vec![50, 51]);
    }

    #[test]
    fn empty_after_filter_returns_err() {
        let vocab = build_vocab();
        assert!(phonemes_to_ids("€£¥", &vocab).is_err());
    }

    #[test]
    fn vocab_has_114_entries() {
        let vocab = build_vocab();
        assert_eq!(vocab.len(), 114);
    }
}
