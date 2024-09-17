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

pub const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/cestef/braise/main/schema/braise.schema.json";

lazy_static! {
    pub static ref ARG_REPLACE_REGEX: Regex = Regex::new(r"\{\d\}").unwrap(); // {0}, {1}, {2}, etc.
    pub static ref ENV_REPLACE_REGEX: Regex = Regex::new(r"\{env\((\w+(?:\:(.+))?)\)\}").unwrap(); // {env(VAR)} or {env(VAR:default)}
}

pub const TASKS_SEPARATOR: char = ',';
