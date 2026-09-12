use std::collections::HashMap;

struct QwertyKeyboard {
    layout: HashMap<char, (i8, i8)>,
}
impl QwertyKeyboard {
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
                let dist_sqr = (f32::from(x1 - x2).powi(2) + f32::from(y1 - y2).powi(2));
                f32::max(0.5, dist_sqr)
            }
            _ => 2.0,
        }
    }
}
