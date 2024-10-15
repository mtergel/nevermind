use regex::Regex;
use std::sync::LazyLock;

pub static USERNAME_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]{3,32}$").unwrap());
