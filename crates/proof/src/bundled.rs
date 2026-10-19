//! Lexicon: Armenian, Georgian and Azerbaijani spelling against Hunspell dictionaries bundled with
//! the app (an installed `hy_AM`, `ka_GE` or `az_AZ` dictionary wins). Suggestions are the
//! dictionary-driven ones Czech uses ([`crate::cs::suggest::suggest_in`]); there are no grammar
//! rules or hyphenation patterns for these languages yet.
//!
//! | Language | Dictionary | Licence |
//! |---|---|---|
//! | Armenian (Eastern) | araktransfer, via wooorm/dictionaries `hy` | MPL 1.1 (of GPL 2 / LGPL 2.1 / MPL 1.1) |
//! | Georgian | `ka_GE.spell` (Gabriel Margiani), via wooorm/dictionaries `ka` | MIT; word lists MIT and CC-BY 4.0 |
//! | Azerbaijani | mozillaz/spellchecker (word list from azerdict.com) | MPL 2.0 |
//!
//! Each dictionary is parsed on first use, so a language costs no memory until text in it is
//! checked.

use std::sync::{Arc, RwLock};

use crate::affix::AffixDict;

/// One bundled language.
pub struct Bundled {
    /// Language name for the log.
    name: &'static str,
    /// File names of an installed dictionary, searched before the bundled one.
    names: &'static [&'static str],
    aff: &'static str,
    dic: &'static str,
    /// The letters the dictionary covers; words in another script are left alone.
    script: fn(char) -> bool,
    dict: RwLock<Option<Arc<AffixDict>>>,
}

/// Armenian letters (and the Armenian ligatures).
pub fn is_armenian(c: char) -> bool {
    matches!(c, '\u{0531}'..='\u{058F}' | '\u{FB13}'..='\u{FB17}')
}

/// Georgian letters (Mkhedruli, Mtavruli, Asomtavruli, Nuskhuri).
pub fn is_georgian(c: char) -> bool {
    matches!(c, '\u{10A0}'..='\u{10FF}' | '\u{1C90}'..='\u{1CBF}' | '\u{2D00}'..='\u{2D2F}')
}

/// Latin letters (Azerbaijani is written in the Latin script, with `ə`).
fn is_latin(c: char) -> bool {
    c.is_alphabetic() && (c < '\u{0250}' || matches!(c, 'ə' | 'Ə') || ('\u{1E00}'..='\u{1EFF}').contains(&c))
}

pub static HY: Bundled = Bundled {
    name: "Armenian",
    names: &["hy_AM", "hy-AM", "hy"],
    aff: include_str!("../../../assets/spelling/hy_AM.aff"),
    dic: include_str!("../../../assets/spelling/hy_AM.dic"),
    script: is_armenian,
    dict: RwLock::new(None),
};

pub static KA: Bundled = Bundled {
    name: "Georgian",
    names: &["ka_GE", "ka-GE", "ka"],
    aff: include_str!("../../../assets/spelling/ka_GE.aff"),
    dic: include_str!("../../../assets/spelling/ka_GE.dic"),
    script: is_georgian,
    dict: RwLock::new(None),
};

pub static AZ: Bundled = Bundled {
    name: "Azerbaijani",
    names: &["az_AZ", "az-AZ", "az_Latn_AZ", "az"],
    aff: include_str!("../../../assets/spelling/az_AZ.aff"),
    dic: include_str!("../../../assets/spelling/az_AZ.dic"),
    script: is_latin,
    dict: RwLock::new(None),
};

/// Words of fewer letters than this written in capitals are taken as abbreviations.
const ABBREVIATION_LETTERS: usize = 6;

impl Bundled {
    /// The dictionary: an installed one, else the bundled one (read on first use).
    pub fn dictionary(&self) -> Arc<AffixDict> {
        if let Some(d) = self.dict.read().unwrap_or_else(|e| e.into_inner()).as_ref() {
            return d.clone();
        }
        let loaded = Arc::new(self.load());
        let mut slot = self.dict.write().unwrap_or_else(|e| e.into_inner());
        slot.get_or_insert(loaded).clone()
    }

    fn load(&self) -> AffixDict {
        if let Some((aff, dic)) = crate::hunspell::find(self.names) {
            match crate::hunspell::load(&aff, &dic) {
                Ok(d) => return d,
                Err(e) => log::warn!("{} spelling: {e}; using the bundled dictionary", self.name),
            }
        }
        self.load_bundled()
    }

    /// The bundled dictionary.
    pub fn load_bundled(&self) -> AffixDict {
        AffixDict::from_hunspell(self.aff, self.dic, &[]).unwrap_or_else(|e| {
            log::warn!("{} spelling: the bundled dictionary failed to load: {e}", self.name);
            AffixDict::default()
        })
    }

    /// Forget the loaded dictionary, so the next check looks for an installed one again.
    pub fn reload(&self) {
        *self.dict.write().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Is `word` spelled correctly? Numbers, single letters, short words in capitals and words in
    /// another script are accepted.
    pub fn is_correct(&self, word: &str) -> bool {
        self.is_correct_with(&self.dictionary(), word)
    }

    /// [`Self::is_correct`] against a given dictionary (an empty one accepts everything).
    pub fn is_correct_with(&self, d: &AffixDict, word: &str) -> bool {
        if d.is_empty() {
            return true;
        }
        // Armenian punctuation sits inside words' Unicode block: the emphasis and question marks
        // (՛ ՜ ՞) and the apostrophe (՚) aren't letters.
        let w = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '-');
        let w: String = w.chars().filter(|c| !matches!(c, '\u{055A}'..='\u{055F}')).collect();
        let w = w.trim_matches('-');
        let n = w.chars().count();
        if n <= 1 || w.chars().any(char::is_numeric) || !w.chars().any(char::is_alphabetic) {
            return true;
        }
        if w.chars().all(|c| !c.is_lowercase()) && n <= ABBREVIATION_LETTERS && w.chars().any(char::is_uppercase) {
            return true;
        }
        if w.chars().any(|c| c.is_alphabetic() && !(self.script)(c)) {
            return true; // another script: not this dictionary's to judge
        }
        let known = |p: &str| {
            d.check(p)
                || crate::in_user_dictionary(&p.to_lowercase())
                // Armenian: the dictionary spells the ligature `և` as `եւ` (Երևան, Երեւան).
                || (p.contains('և') && d.check(&p.replace('և', "եւ")))
        };
        if known(w) {
            return true;
        }
        w.contains('-') && w.split('-').all(|p| p.chars().count() <= 1 || known(p))
    }

    /// Spelling suggestions, best first, at most `max`.
    pub fn suggest(&self, word: &str, max: usize) -> Vec<String> {
        crate::cs::suggest::suggest_in(&self.dictionary(), word, max)
    }

    /// Is a dictionary available?
    pub fn has_dictionary(&self) -> bool {
        !self.dictionary().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_dictionaries_load_and_check_words() {
        for (lang, good, bad) in [
            (&HY, &["Հայաստան", "գիրք", "գրքեր", "և"][..], "գիռք"),
            (&KA, &["საქართველო", "წიგნი", "წიგნები", "თბილისში", "ვცხოვრობ"][..], "საქართვლო"),
            (&AZ, &["kitab", "Azərbaycan", "kitablar", "yaşayıram", "gedirik", "vətənimi", "qoydum", "gələcəyəm", "otağı"][..], "Azərbaycn"),
        ] {
            let d = AffixDict::from_hunspell(lang.aff, lang.dic, &[]).unwrap_or_else(|e| panic!("{}: {e}", lang.name));
            assert!(!d.is_empty(), "{}", lang.name);
            for &w in good {
                assert!(lang.is_correct_with(&d, w), "{}: {w}", lang.name);
            }
            assert!(!lang.is_correct_with(&d, bad), "{}: {bad}", lang.name);
        }
    }

    #[test]
    fn other_scripts_numbers_and_abbreviations_are_left_alone() {
        let d = AffixDict::from_hunspell("SET UTF-8\n", "1\nգիրք\n", &[]).unwrap_or_default();
        for w in ["", "ա", "2026", "ՀՀ", "iPhone", "молоко", "გამარჯობა", "գիրք՞"] {
            assert!(HY.is_correct_with(&d, w), "{w}");
        }
        assert!(!HY.is_correct_with(&d, "գիռք"));
    }
}
