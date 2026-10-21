//! Saisie de dates fixes ; leur présentation finale appartient aux modèles Typst.
use chrono::{Local, NaiveDate};

pub fn today() -> String {
    Local::now().date_naive().to_string()
}
pub fn is_today(value: &str) -> bool {
    let value = value.trim();
    value.eq_ignore_ascii_case("today")
        || value.eq_ignore_ascii_case("aujourd'hui")
        || value.eq_ignore_ascii_case("aujourd’hui")
}
pub fn normalize(value: &mut String) {
    let trimmed = value.trim();
    if is_today(trimmed) {
        *value = today();
    } else if let Some(date) = ["%Y-%m-%d", "%d/%m/%Y"]
        .into_iter()
        .find_map(|format| NaiveDate::parse_from_str(trimmed, format).ok())
    {
        *value = date.to_string();
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    #[test]
    fn aliases_resolve_to_fixed_dates_and_explicit_dates_are_validated() {
        for input in ["today", " TODAY ", "aujourd'hui", "aujourd’hui"] {
            let mut value = input.to_owned();
            normalize(&mut value);
            assert_eq!(value, today());
            assert!(NaiveDate::parse_from_str(&value, "%Y-%m-%d").is_ok());
        }
        for (input, expected) in [
            ("01/02/2026", "2026-02-01"),
            ("29/02/2024", "2024-02-29"),
            (" 2026-10-21 ", "2026-10-21"),
            ("29/02/2025", "29/02/2025"),
            ("mars 2026", "mars 2026"),
            ("", ""),
        ] {
            let mut value = input.to_owned();
            normalize(&mut value);
            assert_eq!(value, expected);
        }
    }
}
