//! 分类输出

use byte_memos_core::model::Category;

use super::{paint, pad, GRAY, YELLOW};

pub fn print_categories(cats: &[Category], json: bool) {
    if json {
        println!("{}", serde_json::to_string_pretty(cats).unwrap());
        return;
    }
    if cats.is_empty() {
        println!("{}", paint(GRAY, "（无分类）"));
        return;
    }
    for c in cats {
        println!(
            "{}  {}",
            paint(YELLOW, &pad(&c.name, 16)),
            paint(GRAY, &format!("(备忘录 {})", c.memo_count)),
        );
    }
}
