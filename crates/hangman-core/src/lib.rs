//! Engine-independent hangman game logic.
//!
//! No I/O, no rendering, no clock: everything here is deterministic and unit-tested.

use std::collections::BTreeSet;

/// Number of wrong guesses that loses the game (head, body, two arms, two legs).
pub const MAX_WRONG: usize = 6;

/// Built-in word list, embedded in the binary so there are no file-path problems.
/// One entry per line: `word|category|clue`.
const WORDS: &str = include_str!("../words.txt");

/// One word with its hint material.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    pub word: &'static str,
    /// Broad group, e.g. "Animal".
    pub category: &'static str,
    /// One short sentence that describes the word without saying it.
    pub clue: &'static str,
}

/// Iterate the built-in entries.
pub fn entries() -> impl Iterator<Item = Entry> {
    WORDS
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut parts = line.splitn(3, '|');
            Entry {
                word: parts.next().unwrap_or_default(),
                category: parts.next().unwrap_or_default(),
                clue: parts.next().unwrap_or_default(),
            }
        })
}

/// Iterate just the words of the built-in list.
pub fn words() -> impl Iterator<Item = &'static str> {
    entries().map(|e| e.word)
}

/// Pick an entry using a caller-supplied seed.
///
/// Different seeds give different entries; the same seed always gives the same one.
pub fn pick_entry(seed: u64) -> Entry {
    // SplitMix64 scramble so nearby seeds (e.g. timestamps) don't pick neighbouring words.
    let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;

    let count = entries().count();
    entries()
        .nth((z % count as u64) as usize)
        .expect("word list is not empty")
}

/// Pick a word using a caller-supplied seed.
pub fn pick_word(seed: u64) -> &'static str {
    pick_entry(seed).word
}

/// How much help the player gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Difficulty {
    /// Shows the word's category and a clue.
    #[default]
    Easy,
    /// Shows the word's category only.
    Medium,
    /// No hints at all.
    Hard,
}

impl Difficulty {
    /// Every level, from most help to least.
    pub const ALL: [Difficulty; 3] = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard];

    pub fn shows_category(self) -> bool {
        self != Difficulty::Hard
    }

    pub fn shows_clue(self) -> bool {
        self == Difficulty::Easy
    }

    pub fn label(self) -> &'static str {
        match self {
            Difficulty::Easy => "Easy",
            Difficulty::Medium => "Medium",
            Difficulty::Hard => "Hard",
        }
    }

    /// Stable lowercase name used when saving settings: `"easy"`, `"medium"` or `"hard"`.
    pub fn name(self) -> &'static str {
        match self {
            Difficulty::Easy => "easy",
            Difficulty::Medium => "medium",
            Difficulty::Hard => "hard",
        }
    }

    /// Parse a saved name (case-insensitive, surrounding spaces ignored).
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim().to_ascii_lowercase().as_str() {
            "easy" => Some(Difficulty::Easy),
            "medium" => Some(Difficulty::Medium),
            "hard" => Some(Difficulty::Hard),
            _ => None,
        }
    }

    /// The following level, wrapping from Hard back to Easy.
    pub fn next(self) -> Self {
        match self {
            Difficulty::Easy => Difficulty::Medium,
            Difficulty::Medium => Difficulty::Hard,
            Difficulty::Hard => Difficulty::Easy,
        }
    }
}

/// Overall state of a game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Playing,
    Won,
    Lost,
}

/// Result of a single guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessResult {
    /// The letter is in the word.
    Hit,
    /// The letter is not in the word (counts as a wrong guess).
    Miss,
    /// The letter was already guessed, or the game is over, or the input isn't A-Z. No change.
    Ignored,
}

/// One game of hangman.
#[derive(Debug, Clone)]
pub struct Game {
    word: String,
    guessed: BTreeSet<char>,
    category: String,
    clue: String,
}

impl Game {
    /// Start a game with the given secret word. Letters are normalized to lowercase;
    /// the word must be non-empty and contain only ASCII letters.
    pub fn new(word: &str) -> Self {
        assert!(
            !word.is_empty() && word.chars().all(|c| c.is_ascii_alphabetic()),
            "word must be non-empty ASCII letters, got {word:?}"
        );
        Self {
            word: word.to_ascii_lowercase(),
            guessed: BTreeSet::new(),
            category: String::new(),
            clue: String::new(),
        }
    }

    /// Start a game from a list entry, keeping its category and clue.
    pub fn from_entry(entry: Entry) -> Self {
        let mut game = Self::new(entry.word);
        game.category = entry.category.to_string();
        game.clue = entry.clue.to_string();
        game
    }

    /// Start a game with an entry chosen from the built-in list.
    pub fn from_seed(seed: u64) -> Self {
        Self::from_entry(pick_entry(seed))
    }

    /// Category of the word, e.g. "Animal". Empty for games made with [`Game::new`].
    pub fn category(&self) -> &str {
        &self.category
    }

    /// One-line clue for the word. Empty for games made with [`Game::new`].
    pub fn clue(&self) -> &str {
        &self.clue
    }

    /// Guess a letter (case-insensitive).
    pub fn guess(&mut self, letter: char) -> GuessResult {
        let letter = letter.to_ascii_lowercase();
        if !letter.is_ascii_lowercase() || self.status() != Status::Playing {
            return GuessResult::Ignored;
        }
        if !self.guessed.insert(letter) {
            return GuessResult::Ignored;
        }
        if self.word.contains(letter) {
            GuessResult::Hit
        } else {
            GuessResult::Miss
        }
    }

    /// The word with unguessed letters hidden, e.g. `"_ a _ _ m a _"`.
    pub fn masked(&self) -> String {
        self.word
            .chars()
            .map(|c| {
                if self.guessed.contains(&c) {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .map(String::from)
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Wrong letters guessed so far, in alphabetical order.
    pub fn wrong_letters(&self) -> Vec<char> {
        self.guessed
            .iter()
            .copied()
            .filter(|c| !self.word.contains(*c))
            .map(|c| c.to_ascii_uppercase())
            .collect()
    }

    /// Number of wrong guesses so far (0..=MAX_WRONG).
    pub fn wrong_count(&self) -> usize {
        self.guessed
            .iter()
            .filter(|c| !self.word.contains(**c))
            .count()
    }

    /// Wrong guesses still allowed before losing.
    pub fn remaining(&self) -> usize {
        MAX_WRONG - self.wrong_count()
    }

    /// Whether this letter (case-insensitive) has been guessed already.
    pub fn is_guessed(&self, letter: char) -> bool {
        self.guessed.contains(&letter.to_ascii_lowercase())
    }

    /// Whether this guessed letter was a hit. `false` if unguessed or a miss.
    pub fn is_hit(&self, letter: char) -> bool {
        let l = letter.to_ascii_lowercase();
        self.guessed.contains(&l) && self.word.contains(l)
    }

    pub fn status(&self) -> Status {
        if self.wrong_count() >= MAX_WRONG {
            Status::Lost
        } else if self.word.chars().all(|c| self.guessed.contains(&c)) {
            Status::Won
        } else {
            Status::Playing
        }
    }

    /// The secret word in uppercase. Show it only when the game is over.
    pub fn reveal(&self) -> String {
        self.word.to_ascii_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_is_fully_hidden() {
        let g = Game::new("rust");
        assert_eq!(g.masked(), "_ _ _ _");
        assert_eq!(g.status(), Status::Playing);
        assert_eq!(g.remaining(), MAX_WRONG);
    }

    #[test]
    fn hit_reveals_every_occurrence() {
        let mut g = Game::new("balloon");
        assert_eq!(g.guess('l'), GuessResult::Hit);
        assert_eq!(g.masked(), "_ _ L L _ _ _");
        assert_eq!(g.guess('o'), GuessResult::Hit);
        assert_eq!(g.masked(), "_ _ L L O O _");
    }

    #[test]
    fn guesses_are_case_insensitive() {
        let mut g = Game::new("Rust");
        assert_eq!(g.guess('R'), GuessResult::Hit);
        assert_eq!(g.guess('r'), GuessResult::Ignored);
        assert!(g.is_guessed('R') && g.is_guessed('r'));
    }

    #[test]
    fn miss_counts_and_is_listed() {
        let mut g = Game::new("rust");
        assert_eq!(g.guess('z'), GuessResult::Miss);
        assert_eq!(g.guess('q'), GuessResult::Miss);
        assert_eq!(g.wrong_count(), 2);
        assert_eq!(g.wrong_letters(), vec!['Q', 'Z']);
        assert_eq!(g.remaining(), MAX_WRONG - 2);
    }

    #[test]
    fn repeated_guess_does_not_double_penalize() {
        let mut g = Game::new("rust");
        g.guess('z');
        assert_eq!(g.guess('z'), GuessResult::Ignored);
        assert_eq!(g.wrong_count(), 1);
    }

    #[test]
    fn non_letters_are_ignored() {
        let mut g = Game::new("rust");
        assert_eq!(g.guess('1'), GuessResult::Ignored);
        assert_eq!(g.guess(' '), GuessResult::Ignored);
        assert_eq!(g.guess('é'), GuessResult::Ignored);
        assert_eq!(g.wrong_count(), 0);
    }

    #[test]
    fn winning() {
        let mut g = Game::new("ab");
        g.guess('a');
        assert_eq!(g.status(), Status::Playing);
        g.guess('b');
        assert_eq!(g.status(), Status::Won);
    }

    #[test]
    fn losing_after_max_wrong() {
        let mut g = Game::new("a");
        for c in "bcdefg".chars() {
            assert_eq!(g.status(), Status::Playing);
            g.guess(c);
        }
        assert_eq!(g.status(), Status::Lost);
        assert_eq!(g.wrong_count(), MAX_WRONG);
        assert_eq!(g.reveal(), "A");
    }

    #[test]
    fn no_guesses_accepted_after_game_over() {
        let mut g = Game::new("a");
        for c in "bcdefg".chars() {
            g.guess(c);
        }
        assert_eq!(g.guess('a'), GuessResult::Ignored);
        assert_eq!(g.status(), Status::Lost);

        let mut g = Game::new("a");
        g.guess('a');
        assert_eq!(g.guess('z'), GuessResult::Ignored);
        assert_eq!(g.wrong_count(), 0);
    }

    #[test]
    fn is_hit_distinguishes_hit_miss_unguessed() {
        let mut g = Game::new("rust");
        g.guess('r');
        g.guess('z');
        assert!(g.is_hit('r'));
        assert!(!g.is_hit('z'));
        assert!(!g.is_hit('u'));
    }

    #[test]
    #[should_panic]
    fn rejects_empty_word() {
        Game::new("");
    }

    #[test]
    #[should_panic]
    fn rejects_non_letters() {
        Game::new("hello world");
    }

    #[test]
    fn word_list_is_valid() {
        let list: Vec<_> = entries().collect();
        assert!(list.len() >= 100);
        for e in &list {
            assert!(
                e.word.len() >= 4 && e.word.chars().all(|c| c.is_ascii_lowercase()),
                "bad word {:?}",
                e.word
            );
            assert!(!e.category.is_empty(), "{:?} has no category", e.word);
            assert!(
                (5..=40).contains(&e.clue.len()),
                "{:?} clue length {} out of range: {:?}",
                e.word,
                e.clue.len(),
                e.clue
            );
            assert!(
                !e.clue.to_ascii_lowercase().contains(e.word),
                "clue for {:?} gives the word away: {:?}",
                e.word,
                e.clue
            );
            assert!(!e.clue.contains('|') && !e.category.contains('|'));
        }
        let unique: BTreeSet<_> = list.iter().map(|e| e.word).collect();
        assert_eq!(unique.len(), list.len(), "duplicate words");
    }

    #[test]
    fn from_seed_carries_hints() {
        let g = Game::from_seed(3);
        assert!(!g.category().is_empty());
        assert!(!g.clue().is_empty());
        // Plain games have no hint data.
        let plain = Game::new("rust");
        assert!(plain.category().is_empty() && plain.clue().is_empty());
    }

    #[test]
    fn difficulty_gates_hints() {
        assert_eq!(Difficulty::default(), Difficulty::Easy);
        let shown = Difficulty::ALL.map(|d| (d.shows_category(), d.shows_clue()));
        assert_eq!(shown, [(true, true), (true, false), (false, false)]);
        assert_eq!(Difficulty::Medium.label(), "Medium");
    }

    #[test]
    fn difficulty_cycles_through_every_level() {
        assert_eq!(Difficulty::Easy.next(), Difficulty::Medium);
        assert_eq!(Difficulty::Medium.next(), Difficulty::Hard);
        assert_eq!(Difficulty::Hard.next(), Difficulty::Easy);
    }

    #[test]
    fn difficulty_names_round_trip() {
        for d in Difficulty::ALL {
            assert_eq!(Difficulty::from_name(d.name()), Some(d));
        }
        assert_eq!(Difficulty::from_name("  HARD "), Some(Difficulty::Hard));
        assert_eq!(Difficulty::from_name("impossible"), None);
        assert_eq!(Difficulty::from_name(""), None);
    }

    #[test]
    fn pick_word_is_deterministic_and_varied() {
        assert_eq!(pick_word(42), pick_word(42));
        let picks: BTreeSet<_> = (0..200).map(pick_word).collect();
        assert!(picks.len() > 50, "only {} distinct words", picks.len());
    }

    #[test]
    fn from_seed_makes_playable_game() {
        let g = Game::from_seed(7);
        assert_eq!(g.status(), Status::Playing);
        assert!(g.masked().contains('_'));
    }
}
