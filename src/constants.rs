use const_format::formatcp;
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

pub const SCHEMA_URL: &'static str =
    "https://raw.githubusercontent.com/cestef/braise/main/braise.schema.json";

pub const DEFAULT_CONFIG: &str = formatcp!(
    r#"#:schema {}
[echo]
command = "echo Hello, world!"
description = "Prints 'Hello, world!' to the console""#,
    SCHEMA_URL
);

lazy_static! {
    pub static ref ARG_REPLACE_REGEX: Regex = Regex::new(r"\{\d\}").unwrap(); // {0}, {1}, {2}, etc.
    pub static ref ENV_REPLACE_REGEX: Regex = Regex::new(r"\{env\((\w+(?:\:(.+))?)\)\}").unwrap(); // {env(VAR)} or {env(VAR:default)}
}

pub const TASKS_SEPARATOR: char = ',';
