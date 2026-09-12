use std::collections::HashMap;

static SUFFIXES: [&'static str; 8] = ["s", "es", "ly", "ed", "ing", "y", "tion", "er"];

trait Keyboard {
    fn new() -> Self;
    fn distance(&self, c1: char, c2: char) -> f32;
}

struct QwertyKeyboard {
    layout: HashMap<char, (i8, i8)>,
}
impl Keyboard for QwertyKeyboard {
    fn new() -> Self {
        let mut layout = HashMap::new();
        let rows = vec!["qwertyuiop", "asdfghjkl", "zxcvbnm"];
        for (r, row) in rows.iter().enumerate() {
            for (c, ch) in row.chars().enumerate() {
                layout.insert(ch, (c as i8, r as i8));
            }
        }
        QwertyKeyboard { layout: layout }
    }

    fn distance(&self, c1: char, c2: char) -> f32 {
        let p1 = self.layout.get(&c1);
        let p2 = self.layout.get(&c2);

        match (p1, p2) {
            (Some((x1, y1)), Some((x2, y2))) => {
                (f32::from(x1 - x2).powi(2) + f32::from(y1 - y2).powi(2)).sqrt()
            }
            // Unknown characters (special characters, numbers, etc.) map to a default distance
            _ => 2.0,
        }
    }
}

fn get_substitution_cost(c1: char, c2: char, keyboard: &impl Keyboard) -> f32 {
    keyboard.distance(c1.to_ascii_lowercase(), c2.to_ascii_lowercase())
}

fn get_deletion_cost(word: &[char], i: usize) -> f32 {
    let ch = word[i];
    // Discount for deleting a plural 's' at the end of a word
    if ch == 's' && i == word.len() - 1 {
        return 0.3;
    }
    // Discount for deleting a duplicated letter
    if i > 0 && word[i - 1] == ch {
        return 0.2;
    }
    if i < word.len() - 1 && word[i + 1] == ch {
        return 0.2;
    }
    2.0
}

fn get_insertion_cost(word: &[char], i: usize) -> f32 {
    let ch = word[i];
    // Discount for inserting a plural 's' at the end of a word
    if ch == 's' && i == word.len() - 1 {
        return 0.3;
    }
    // Discount for inserting a duplicated letter
    if i > 0 && word[i - 1] == ch {
        return 0.2;
    }
    if i < word.len() - 1 && word[i + 1] == ch {
        return 0.2;
    }
    2.0
}

fn weighted_edit_distance(target: &str, sample: &str) -> f32 {
    let s1: Vec<char> = sample.chars().collect();
    let s2: Vec<char> = target.chars().collect();
    let m = s1.len();
    let n = s2.len();

    let keyboard = QwertyKeyboard::new();

    let mut dp = vec![vec![0.0; n + 1]; m + 1];
    for i in 1..=m {
        dp[i][0] = dp[i - 1][0] + get_deletion_cost(&s1, i - 1);
    }
    for j in 1..=n {
        dp[0][j] = dp[0][j - 1] + get_insertion_cost(&s2, j - 1);
    }

    for i in 1..=m {
        for j in 1..=n {
            let cost_sub =
                dp[i - 1][j - 1] + get_substitution_cost(s1[i - 1], s2[j - 1], &keyboard);
            let cost_del = dp[i - 1][j] + get_deletion_cost(&s1, i - 1);
            let cost_ins = dp[i][j - 1] + get_insertion_cost(&s2, j - 1);

            dp[i][j] = f32::min(cost_sub, f32::min(cost_del, cost_ins));
        }
    }

    dp[m][n]
}

pub fn close_enough(target: &str, sample: &str) -> bool {
    // Strip suffix if applicable
    let stripped_sample = SUFFIXES
        .iter()
        .find_map(|suffix| sample.strip_suffix(suffix))
        .unwrap_or(sample);

    let target_len = target.len();
    let sample_len = stripped_sample.len();

    if !(target_len / 2 <= sample_len && sample_len <= target_len * 2) {
        // short circuit: sample and target length vastly differ.
        return false;
    }

    let dist = weighted_edit_distance(target, stripped_sample);
    dist <= 2.5
}
