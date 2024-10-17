const GENERATOR_URL: &str =
    "https://api.dicebear.com/9.x/thumbs/svg?backgroundColor=b6e3f4,c0aede,d1d4f9";

pub fn generate_avatar(seed: &str) -> String {
    format!("{}&seed={}", GENERATOR_URL, seed)
}
