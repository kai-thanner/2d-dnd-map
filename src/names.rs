use crate::world::SettlementKind;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;

const PRE: &[&str] = &[
    "Ald", "Bran", "Dun", "El", "Fal", "Gar", "Hel", "Kal", "Lor", "Mor", "Nor", "Rav", "Sil",
    "Tor", "Ul", "Val", "Wil", "Yar", "Zan", "Ash", "Kra",
];
const MID: &[&str] = &["a", "e", "i", "o", "ar", "en", "or", "ia", "eth", "ow", "ui"];
const SUF_CITY: &[&str] = &["burg", "stadt", "hafen", "thal", "furt", "mark"];
const SUF_VILLAGE: &[&str] = &["dorf", "heim", "feld", "hain", "brück", "weiler", "au"];

pub fn settlement(rng: &mut StdRng, kind: SettlementKind) -> String {
    let suf = match kind {
        SettlementKind::City => SUF_CITY,
        SettlementKind::Village => SUF_VILLAGE,
    };
    format!(
        "{}{}{}",
        PRE.choose(rng).unwrap(),
        MID.choose(rng).unwrap(),
        suf.choose(rng).unwrap()
    )
}

const DUNGEON_KIND: &[&str] = &[
    "Krypta", "Höhle", "Ruine", "Katakomben", "Feste", "Grabmal",
];
const DUNGEON_SUBJ: &[&str] = &[
    "Aschenbrand",
    "Nachtfalke",
    "Schattengrund",
    "Eisenkrone",
    "Sturmklippe",
    "Moorherz",
    "Knochenfeld",
    "Winterzahn",
];

pub fn dungeon(rng: &mut StdRng) -> String {
    format!(
        "{} {}",
        DUNGEON_KIND.choose(rng).unwrap(),
        DUNGEON_SUBJ.choose(rng).unwrap()
    )
}

const INN_ADJ: &[&str] = &[
    "Goldenen",
    "Roten",
    "Silbernen",
    "Letzten",
    "Fauchenden",
    "Tanzenden",
    "Verwunschenen",
];
const INN_NOUN: &[&str] = &[
    "Drachen", "Einhorn", "Krone", "Raben", "Fass", "Stern", "Wolf", "Hufeisen",
];

pub fn inn(rng: &mut StdRng) -> String {
    format!(
        "Zum {} {}",
        INN_ADJ.choose(rng).unwrap(),
        INN_NOUN.choose(rng).unwrap()
    )
}

const MERCHANT_FIRST: &[&str] = &[
    "Bram", "Tilda", "Osric", "Maren", "Perrin", "Ysolt", "Kargan", "Eda",
];
const MERCHANT_LAST: &[&str] = &[
    "Kesselfuß", "Goldhand", "Wandergut", "Schmied", "Flink", "Sternenblick",
];
const GOODS: &[&str] = &[
    "Waffen",
    "Gewürze",
    "Seide",
    "Schnaps",
    "Landkarten",
    "Zaubertränke",
    "Edelsteine",
    "Lederwaren",
];

pub fn merchant(rng: &mut StdRng) -> (String, String) {
    (
        format!(
            "{} {}",
            MERCHANT_FIRST.choose(rng).unwrap(),
            MERCHANT_LAST.choose(rng).unwrap()
        ),
        GOODS.choose(rng).unwrap().to_string(),
    )
}
