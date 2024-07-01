use lazy_static::lazy_static;
use regex::Regex;

pub const FILE_NAMES: [&str; 8] = [
    "braise",
    "braisefile",
    ".braisefile",
    ".braise",
    "Braisefile",
    "Braise",
    ".braiserc",
    "braiserc",
];

lazy_static! {
    pub static ref REPLACE_REGEX: Regex = Regex::new(r"\{\d\}").unwrap();
}
