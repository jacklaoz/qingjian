//! 符号候选：按敲的输入码查，插在第 2 位。

use super::*;

use crate::candidate::ExtraCandidates;
use crate::symbol::SymbolTable;

/// 一张够用的小表。`kaifa` 在样例词库里有词（开发），`duigou` 没有——两种情形都要测到。
fn symbols() -> SymbolTable {
    SymbolTable::parse("duigou\t对勾\t✔ ✓ ☑\nkaifa\t开发\t→ ←\n").unwrap()
}

/// 名字不在词库里的符号也打得出来——这正是符号不跟 emoji 共用一张表的原因。
#[test]
fn symbols_come_from_the_typed_code_not_from_a_word() {
    let mut engine = engine().with_symbols(symbols());
    engine.set_input("duigou");
    let items = engine.query().unwrap().candidates.items;
    let symbol = items.iter().find(|c| c.text == "✔").expect("候选里该有 ✔");
    assert_eq!(symbol.kind, CandidateKind::Emoji);
    assert_eq!(symbol.reading.as_deref(), Some("对勾"));
}

/// 插在第 2 位：首选仍然是词。打 `kaifa` 多半是要「开发」，不是要符号。
#[test]
fn symbols_take_the_second_slot_and_leave_the_first_to_words() {
    let mut engine = engine().with_symbols(symbols());
    engine.set_input("kaifa");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "开发", "首选要留给词");
    assert_eq!(items[1].text, "→");
    assert_eq!(items[2].text, "←");
}

/// 输入压根没有候选时（名字不是词、拼音也读不出来），符号就是首选——不然它没地方待。
#[test]
fn symbols_lead_when_there_is_nothing_else() {
    let mut engine = engine().with_symbols(symbols());
    engine.set_input("duigou");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "✔");
}

/// 一次最多三个，别把第一页占满。
#[test]
fn at_most_three_symbols() {
    let table = SymbolTable::parse("duigou\t对勾\t✔ ✓ ☑ ☑︎ ✅ 🗸\n").unwrap();
    let mut engine = engine().with_symbols(table);
    engine.set_input("duigou");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(
        items
            .iter()
            .filter(|c| c.reading.as_deref() == Some("对勾"))
            .count(),
        3
    );
}

/// emoji 与符号合起来最多占四格：第一页一共才 9 格，挑符号不该把词挤到第二页。
#[test]
fn emoji_and_symbols_share_one_budget() {
    let mut engine = engine()
        .with_emoji(EmojiTable::parse("开发\t🛠 ⚙\n开饭\t🍚\n").unwrap())
        .with_symbols(SymbolTable::parse("kaifa\t开发\t✔ ✓ ☑\n").unwrap());
    engine.set_input("kaifa");
    let items = engine.query().unwrap().candidates.items;
    let extras = items
        .iter()
        .filter(|c| c.kind == CandidateKind::Emoji)
        .count();
    assert!(extras <= 4, "emoji 加符号最多四格，实际 {extras}");
    assert!(items.iter().any(|c| c.text == "✔"), "符号至少出一个");
}

/// 只认完整相等：打到一半不冒符号。
#[test]
fn a_half_typed_code_shows_nothing() {
    let mut engine = engine().with_symbols(symbols());
    engine.set_input("duig");
    let items = engine.query().unwrap().candidates.items;
    assert!(items.iter().all(|c| c.text != "✔"));
}

/// 名字正好是词库里的词时不出两遍：emoji 那条路已经把它挂在词后面了。
#[test]
fn a_symbol_already_in_the_list_is_not_inserted_twice() {
    let mut engine = engine()
        .with_emoji(EmojiTable::parse("开发\t🛠\n").unwrap())
        .with_symbols(SymbolTable::parse("kaifa\t开发\t🛠 ⚙\n").unwrap());
    engine.set_input("kaifa");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(
        items.iter().filter(|c| c.text == "🛠").count(),
        1,
        "🛠 只该出一次"
    );
    assert!(items.iter().any(|c| c.text == "⚙"), "另一个符号照常出");
}

/// 开关：`symbol` 只出符号，`emoji` 只出 emoji，`off` 两样都不出。
#[test]
fn the_switch_selects_what_shows_up() {
    let build = || {
        engine()
            .with_emoji(EmojiTable::parse("开发\t🛠\n").unwrap())
            .with_symbols(SymbolTable::parse("kaifa\t开发\t✌\n").unwrap())
    };
    let texts = |engine: &mut Engine| {
        engine.set_input("kaifa");
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
    };

    let mut engine = build();
    engine.set_extras(ExtraCandidates::Symbol);
    let items = texts(&mut engine);
    assert!(items.contains(&"✌".to_owned()) && !items.contains(&"🛠".to_owned()));

    let mut engine = build();
    engine.set_extras(ExtraCandidates::Emoji);
    let items = texts(&mut engine);
    assert!(!items.contains(&"✌".to_owned()) && items.contains(&"🛠".to_owned()));

    let mut engine = build();
    engine.set_extras(ExtraCandidates::Off);
    let items = texts(&mut engine);
    assert!(!items.contains(&"✌".to_owned()) && !items.contains(&"🛠".to_owned()));
}

/// 符号能上屏，且吃掉整段输入。
#[test]
fn committing_a_symbol_clears_the_composition() {
    let mut engine = engine().with_symbols(symbols());
    engine.set_input("duigou");
    let symbol = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "✔")
        .unwrap();
    assert_eq!(engine.commit(&symbol), "✔");
    assert!(engine.composition().is_empty());
}
