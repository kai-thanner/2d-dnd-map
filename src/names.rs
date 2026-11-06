use crate::world::SettlementTier;
use rand::rngs::StdRng;
use rand::seq::IndexedRandom;

const PRE: &[&str] = &[
    "Ald", "Bran", "Dun", "El", "Fal", "Gar", "Hel", "Kal", "Lor", "Mor", "Nor", "Rav", "Sil",
    "Tor", "Ul", "Val", "Wil", "Yar", "Zan", "Ash", "Kra",
];
const MID: &[&str] = &["a", "e", "i", "o", "ar", "en", "or", "ia", "eth", "ow", "ui"];
const SUF_GEHOEFT: &[&str] = &["hof", "gut", "au"];
const SUF_DORF: &[&str] = &["dorf", "heim", "feld", "hain", "weiler"];
const SUF_KLEINSTADT: &[&str] = &["flecken", "furt", "brück"];
const SUF_STADT: &[&str] = &["burg", "stadt", "thal", "mark"];
const SUF_KAISERSTADT: &[&str] = &["stadt", "burg", "krone"];
const SUF_HAFENSTADT: &[&str] = &["hafen", "strand", "pier"];

pub fn settlement(rng: &mut StdRng, population: u32, near_ocean: bool) -> String {
    let tier = SettlementTier::from_population(population, near_ocean);
    let suf: &[&str] = match tier {
        SettlementTier::Gehoeft => SUF_GEHOEFT,
        SettlementTier::Dorf => SUF_DORF,
        SettlementTier::Kleinstadt => SUF_KLEINSTADT,
        SettlementTier::Stadt => SUF_STADT,
        SettlementTier::Kaiserstadt => SUF_KAISERSTADT,
        SettlementTier::Hafenstadt => SUF_HAFENSTADT,
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

const FAMOUS_MERCHANTS: &[&str] = &[
    "Baron Kazimir Goldhand",
    "Mira Sturmfeather",
    "Gilbert der Weitgereiste",
    "Sister Aurelia vom heiligen Markt",
    "Volo Knochenkralle",
    "Lady Seraphine Dreiblatt",
];

pub fn famous_merchant(rng: &mut StdRng) -> String {
    FAMOUS_MERCHANTS.choose(rng).unwrap().to_string()
}

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
