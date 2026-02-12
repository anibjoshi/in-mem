//! Porter stemmer implementation (no external dependencies)
//!
//! Implements the Porter stemming algorithm as described in:
//! Porter, M.F. "An algorithm for suffix stripping." Program 14.3 (1980): 130-137.
//!
//! This operates on ASCII lowercase input only. Non-ASCII words are returned unchanged.

/// Stem a word using the Porter algorithm.
///
/// Input must be lowercase ASCII. Non-ASCII or very short words are returned as-is.
///
/// # Examples
///
/// ```
/// use strata_engine::search::stemmer::stem;
///
/// assert_eq!(stem("running"), "run");
/// assert_eq!(stem("caresses"), "caress");
/// assert_eq!(stem("generalization"), "gener");
/// ```
pub fn stem(word: &str) -> String {
    // Words <= 2 chars or containing non-ASCII are not stemmed
    if word.len() <= 2 || !word.is_ascii() {
        return word.to_string();
    }

    let mut w = word.to_string();
    let j = 0usize; // tracks the "stem boundary" used by step1b

    // Step 1a
    w = step1a(&w);

    // Step 1b (may set j for follow-up)
    let (w1b, did_second_or_third) = step1b(&w);
    w = w1b;

    // Step 1b follow-up
    if did_second_or_third {
        w = step1b_fixup(&w);
    }

    // Step 1c
    w = step1c(&w);

    // Step 2
    w = step2(&w);

    // Step 3
    w = step3(&w);

    // Step 4
    w = step4(&w);

    // Step 5a
    w = step5a(&w);

    // Step 5b
    w = step5b(&w);

    let _ = j; // suppress unused warning
    w
}

// ==========================================================================
// Helpers
// ==========================================================================

/// Is the byte at position i a consonant?
fn is_consonant(b: &[u8], i: usize) -> bool {
    match b[i] {
        b'a' | b'e' | b'i' | b'o' | b'u' => false,
        b'y' => {
            if i == 0 {
                true
            } else {
                !is_consonant(b, i - 1)
            }
        }
        _ => true,
    }
}

/// Compute the "measure" m of a word (number of VC sequences).
///
/// A word is partitioned into [C](VC)^m[V], where C = consonant sequence,
/// V = vowel sequence. m is the number of VC pairs.
fn measure(word: &str) -> usize {
    let b = word.as_bytes();
    let len = b.len();
    if len == 0 {
        return 0;
    }

    let mut i = 0;
    let mut m = 0;

    // skip initial consonants
    while i < len && is_consonant(b, i) {
        i += 1;
    }

    loop {
        if i >= len {
            return m;
        }
        // skip vowels
        while i < len && !is_consonant(b, i) {
            i += 1;
        }
        if i >= len {
            return m;
        }
        // skip consonants
        while i < len && is_consonant(b, i) {
            i += 1;
        }
        m += 1;
    }
}

/// Does the stem contain a vowel?
fn contains_vowel(word: &str) -> bool {
    let b = word.as_bytes();
    for i in 0..b.len() {
        if !is_consonant(b, i) {
            return true;
        }
    }
    false
}

/// Does the word end with a double consonant?
fn ends_double_consonant(word: &str) -> bool {
    let b = word.as_bytes();
    let len = b.len();
    if len < 2 {
        return false;
    }
    b[len - 1] == b[len - 2] && is_consonant(b, len - 1)
}

/// Does the word end with consonant-vowel-consonant, where the final
/// consonant is not w, x, or y?
fn ends_cvc(word: &str) -> bool {
    let b = word.as_bytes();
    let len = b.len();
    if len < 3 {
        return false;
    }
    let c2 = b[len - 1];
    if !is_consonant(b, len - 1) || !is_consonant(b, len - 3) {
        return false;
    }
    if is_consonant(b, len - 2) {
        return false;
    }
    // final consonant must not be w, x, y
    !matches!(c2, b'w' | b'x' | b'y')
}

/// Get the stem after removing suffix of given length.
fn stem_before(word: &str, suffix_len: usize) -> &str {
    &word[..word.len() - suffix_len]
}

// ==========================================================================
// Steps
// ==========================================================================

/// Step 1a: Plurals
fn step1a(word: &str) -> String {
    if word.ends_with("sses") {
        return format!("{}ss", stem_before(word, 4));
    }
    if word.ends_with("ies") {
        return format!("{}i", stem_before(word, 3));
    }
    if word.ends_with("ss") {
        return word.to_string();
    }
    if word.ends_with('s') {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

/// Step 1b: Past tense / gerunds
/// Returns (word, did_second_or_third_rule) — the flag indicates whether
/// the step1b fixup should run.
fn step1b(word: &str) -> (String, bool) {
    if word.ends_with("eed") {
        let stem = stem_before(word, 3);
        if measure(stem) > 0 {
            return (format!("{}ee", stem), false);
        }
        return (word.to_string(), false);
    }
    if word.ends_with("ed") {
        let stem = stem_before(word, 2);
        if contains_vowel(stem) {
            return (stem.to_string(), true);
        }
        return (word.to_string(), false);
    }
    if word.ends_with("ing") {
        let stem = stem_before(word, 3);
        if contains_vowel(stem) {
            return (stem.to_string(), true);
        }
        return (word.to_string(), false);
    }
    (word.to_string(), false)
}

/// Step 1b fixup: after removing -ed or -ing
fn step1b_fixup(word: &str) -> String {
    if word.ends_with("at") || word.ends_with("bl") || word.ends_with("iz") {
        return format!("{}e", word);
    }
    if ends_double_consonant(word) {
        let last = word.as_bytes()[word.len() - 1];
        if last != b'l' && last != b's' && last != b'z' {
            return word[..word.len() - 1].to_string();
        }
    }
    if measure(word) == 1 && ends_cvc(word) {
        return format!("{}e", word);
    }
    word.to_string()
}

/// Step 1c: Y -> I
fn step1c(word: &str) -> String {
    if word.ends_with('y') {
        let stem = stem_before(word, 1);
        if contains_vowel(stem) {
            return format!("{}i", stem);
        }
    }
    word.to_string()
}

/// Step 2: Map double suffixes to single
fn step2(word: &str) -> String {
    // Order matters: longest suffixes first within each ending letter group
    let rules: &[(&str, &str)] = &[
        ("ational", "ate"),
        ("tional", "tion"),
        ("enci", "ence"),
        ("anci", "ance"),
        ("izer", "ize"),
        ("bli", "ble"),
        ("alli", "al"),
        ("entli", "ent"),
        ("eli", "e"),
        ("ousli", "ous"),
        ("ization", "ize"),
        ("ation", "ate"),
        ("ator", "ate"),
        ("alism", "al"),
        ("iveness", "ive"),
        ("fulness", "ful"),
        ("ousness", "ous"),
        ("aliti", "al"),
        ("iviti", "ive"),
        ("biliti", "ble"),
        ("logi", "log"),
    ];

    for &(suffix, replacement) in rules {
        if word.ends_with(suffix) {
            let stem = stem_before(word, suffix.len());
            if measure(stem) > 0 {
                return format!("{}{}", stem, replacement);
            }
            return word.to_string();
        }
    }
    word.to_string()
}

/// Step 3: Handle -icate, -ative, -alize, etc.
fn step3(word: &str) -> String {
    let rules: &[(&str, &str)] = &[
        ("icate", "ic"),
        ("ative", ""),
        ("alize", "al"),
        ("iciti", "ic"),
        ("ical", "ic"),
        ("ful", ""),
        ("ness", ""),
    ];

    for &(suffix, replacement) in rules {
        if word.ends_with(suffix) {
            let stem = stem_before(word, suffix.len());
            if measure(stem) > 0 {
                return format!("{}{}", stem, replacement);
            }
            return word.to_string();
        }
    }
    word.to_string()
}

/// Step 4: Remove suffixes where m > 1
fn step4(word: &str) -> String {
    // Special case: -ion requires preceding s or t
    let suffixes: &[&str] = &[
        "al", "ance", "ence", "er", "ic", "able", "ible", "ant", "ement", "ment", "ent", "ion",
        "ou", "ism", "ate", "iti", "ous", "ive", "ize",
    ];

    for &suffix in suffixes {
        if word.ends_with(suffix) {
            let stem = stem_before(word, suffix.len());
            if suffix == "ion" {
                // -ion requires the stem to end with s or t
                if measure(stem) > 1
                    && !stem.is_empty()
                    && matches!(stem.as_bytes()[stem.len() - 1], b's' | b't')
                {
                    return stem.to_string();
                }
            } else if measure(stem) > 1 {
                return stem.to_string();
            }
            return word.to_string();
        }
    }
    word.to_string()
}

/// Step 5a: Remove trailing -e
fn step5a(word: &str) -> String {
    if word.ends_with('e') {
        let stem = stem_before(word, 1);
        let m = measure(stem);
        if m > 1 {
            return stem.to_string();
        }
        if m == 1 && !ends_cvc(stem) {
            return stem.to_string();
        }
    }
    word.to_string()
}

/// Step 5b: -ll -> -l when m > 1
fn step5b(word: &str) -> String {
    if word.ends_with("ll") && measure(word) > 1 {
        return word[..word.len() - 1].to_string();
    }
    word.to_string()
}

// ==========================================================================
// Tests
// ==========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // Helper tests
    // ------------------------------------------------------------------

    #[test]
    fn test_measure() {
        assert_eq!(measure("tr"), 0);
        assert_eq!(measure("ee"), 0);
        assert_eq!(measure("tree"), 0);
        assert_eq!(measure("y"), 0);
        assert_eq!(measure("by"), 0);
        assert_eq!(measure("trouble"), 1);
        assert_eq!(measure("oats"), 1);
        assert_eq!(measure("trees"), 1);
        assert_eq!(measure("ivy"), 1);
        assert_eq!(measure("troubles"), 2);
        assert_eq!(measure("private"), 2);
        assert_eq!(measure("oaten"), 2);
        assert_eq!(measure("orrery"), 2);
    }

    #[test]
    fn test_contains_vowel() {
        assert!(contains_vowel("apple"));
        assert!(contains_vowel("a"));
        assert!(!contains_vowel("str"));
        assert!(!contains_vowel("b"));
    }

    #[test]
    fn test_ends_double_consonant() {
        assert!(ends_double_consonant("fall"));
        assert!(ends_double_consonant("hiss"));
        assert!(ends_double_consonant("fizz"));
        assert!(!ends_double_consonant("fail"));
        assert!(!ends_double_consonant("fee"));
    }

    #[test]
    fn test_ends_cvc() {
        assert!(ends_cvc("wil")); // c-v-c, l is not w/x/y
        assert!(ends_cvc("hop"));
        assert!(!ends_cvc("how")); // ends with w
        assert!(!ends_cvc("box")); // ends with x
        assert!(!ends_cvc("toy")); // ends with y
    }

    // ------------------------------------------------------------------
    // Step 1a tests
    // ------------------------------------------------------------------

    #[test]
    fn test_step1a() {
        assert_eq!(stem("caresses"), "caress");
        assert_eq!(stem("ponies"), "poni");
        assert_eq!(stem("ties"), "ti");
        assert_eq!(stem("caress"), "caress");
        assert_eq!(stem("cats"), "cat");
    }

    // ------------------------------------------------------------------
    // Step 1b tests
    // ------------------------------------------------------------------

    #[test]
    fn test_step1b() {
        assert_eq!(stem("feed"), "feed");
        assert_eq!(stem("agreed"), "agre");
        assert_eq!(stem("plastered"), "plaster");
        assert_eq!(stem("bled"), "bled");
        assert_eq!(stem("motoring"), "motor");
        assert_eq!(stem("sing"), "sing");
    }

    #[test]
    fn test_step1b_fixup() {
        assert_eq!(stem("conflated"), "conflat");
        assert_eq!(stem("troubled"), "troubl");
        assert_eq!(stem("sized"), "size");
        assert_eq!(stem("hopping"), "hop");
        assert_eq!(stem("tanned"), "tan");
        assert_eq!(stem("falling"), "fall");
        assert_eq!(stem("hissing"), "hiss");
        assert_eq!(stem("fizzing"), "fizz");
        assert_eq!(stem("failing"), "fail");
        assert_eq!(stem("filing"), "file");
    }

    // ------------------------------------------------------------------
    // Step 1c tests
    // ------------------------------------------------------------------

    #[test]
    fn test_step1c() {
        assert_eq!(stem("happy"), "happi");
        assert_eq!(stem("sky"), "sky");
    }

    // ------------------------------------------------------------------
    // Full stemming tests — Porter's published test cases
    // ------------------------------------------------------------------

    #[test]
    fn test_porter_standard_cases() {
        let cases = vec![
            ("caresses", "caress"),
            ("ponies", "poni"),
            ("ties", "ti"),
            ("caress", "caress"),
            ("cats", "cat"),
            ("feed", "feed"),
            ("agreed", "agre"),
            ("plastered", "plaster"),
            ("bled", "bled"),
            ("motoring", "motor"),
            ("sing", "sing"),
            ("conflated", "conflat"),
            ("troubled", "troubl"),
            ("sized", "size"),
            ("hopping", "hop"),
            ("tanned", "tan"),
            ("falling", "fall"),
            ("hissing", "hiss"),
            ("fizzing", "fizz"),
            ("failing", "fail"),
            ("filing", "file"),
            ("happy", "happi"),
            ("sky", "sky"),
            ("relational", "relat"),
            ("conditional", "condit"),
            ("rational", "ration"),
            ("valenci", "valenc"),
            ("hesitanci", "hesit"),
            ("digitizer", "digit"),
            ("conformabli", "conform"),
            ("radicalli", "radic"),
            ("differentli", "differ"),
            ("vileli", "vile"),
            ("analogousli", "analog"),
            ("vietnamization", "vietnam"),
            ("predication", "predic"),
            ("operator", "oper"),
            ("feudalism", "feudal"),
            ("decisiveness", "decis"),
            ("hopefulness", "hope"),
            ("callousness", "callous"),
            ("formaliti", "formal"),
            ("sensitiviti", "sensit"),
            ("sensibiliti", "sensibl"),
            ("triplicate", "triplic"),
            ("formative", "form"),
            ("formalize", "formal"),
            ("electriciti", "electr"),
            ("electrical", "electr"),
            ("hopeful", "hope"),
            ("goodness", "good"),
            ("revival", "reviv"),
            ("allowance", "allow"),
            ("inference", "infer"),
            ("airliner", "airlin"),
            ("gyroscopic", "gyroscop"),
            ("adjustable", "adjust"),
            ("defensible", "defens"),
            ("irritant", "irrit"),
            ("replacement", "replac"),
            ("adjustment", "adjust"),
            ("dependent", "depend"),
            ("adoption", "adopt"),
            ("homologou", "homolog"),
            ("communism", "commun"),
            ("activate", "activ"),
            ("angulariti", "angular"),
            ("homologous", "homolog"),
            ("effective", "effect"),
            ("bowdlerize", "bowdler"),
            ("probate", "probat"),
            ("rate", "rate"),
            ("cease", "ceas"),
            ("controll", "control"),
            ("roll", "roll"),
        ];

        for (input, expected) in cases {
            let result = stem(input);
            assert_eq!(
                result, expected,
                "stem({:?}) = {:?}, expected {:?}",
                input, result, expected
            );
        }
    }

    // ------------------------------------------------------------------
    // Edge cases
    // ------------------------------------------------------------------

    #[test]
    fn test_short_words() {
        assert_eq!(stem("a"), "a");
        assert_eq!(stem("be"), "be");
        assert_eq!(stem(""), "");
    }

    #[test]
    fn test_non_ascii() {
        assert_eq!(stem("café"), "café");
    }

    // ------------------------------------------------------------------
    // IR-relevant cases
    // ------------------------------------------------------------------

    #[test]
    fn test_medical_terms() {
        // These should all stem to similar roots
        assert_eq!(stem("treatment"), stem("treatments"));
        assert_eq!(stem("infection"), stem("infections"));
        assert_eq!(stem("clinical"), stem("clinically"));
        assert_eq!(stem("patient"), stem("patients"));
    }

    #[test]
    fn test_common_ir_words() {
        assert_eq!(stem("running"), "run");
        assert_eq!(stem("runs"), "run");
        assert_eq!(stem("runner"), "runner");
        assert_eq!(stem("generalization"), "gener");
        assert_eq!(stem("generalizations"), "gener");
    }

    /// Validate against Martin Porter's canonical 23,531-word test vocabulary.
    ///
    /// Test data from https://tartarus.org/martin/PorterStemmer/
    /// Files: tests/data/porter_voc.txt (input), tests/data/porter_output.txt (expected)
    #[test]
    fn test_porter_canonical_vocabulary() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let voc_path = format!("{}/tests/data/porter_voc.txt", manifest_dir);
        let out_path = format!("{}/tests/data/porter_output.txt", manifest_dir);

        let voc = match std::fs::read_to_string(&voc_path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Skipping canonical test: {} not found", voc_path);
                return;
            }
        };
        let out = std::fs::read_to_string(&out_path).unwrap();

        let inputs: Vec<&str> = voc.lines().collect();
        let expected: Vec<&str> = out.lines().collect();
        assert_eq!(inputs.len(), expected.len());

        let mut failures = Vec::new();
        for (input, exp) in inputs.iter().zip(expected.iter()) {
            let got = stem(input);
            if got != *exp {
                failures.push(format!("{:?} => {:?} (expected {:?})", input, got, exp));
            }
        }

        if !failures.is_empty() {
            panic!(
                "{} / {} failures ({:.2}%):\n{}",
                failures.len(),
                inputs.len(),
                failures.len() as f64 / inputs.len() as f64 * 100.0,
                failures[..failures.len().min(30)].join("\n")
            );
        }
    }
}
