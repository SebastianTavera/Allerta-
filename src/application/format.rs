//! Formato de fechas en español, compartido por la UI y los correos.

use chrono::{Datelike, Local, TimeZone, Timelike};

const MONTHS: [&str; 12] =
    ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sep", "oct", "nov", "dic"];

/// `11 nov 2026 · 14:32` en hora local.
pub fn datetime(timestamp: i64) -> String {
    match Local.timestamp_opt(timestamp, 0).single() {
        Some(dt) => format!(
            "{} {} {} · {:02}:{:02}",
            dt.day(),
            MONTHS[dt.month0() as usize],
            dt.year(),
            dt.hour(),
            dt.minute()
        ),
        None => "fecha desconocida".to_owned(),
    }
}

/// Tiempo relativo corto: `ahora`, `hace 5 min`, `hace 3 h`, `ayer`…
pub fn relative(timestamp: i64, now: i64) -> String {
    let elapsed = (now - timestamp).max(0);
    match elapsed {
        0..=59 => "ahora".to_owned(),
        60..=3_599 => format!("hace {} min", elapsed / 60),
        3_600..=86_399 => format!("hace {} h", elapsed / 3_600),
        86_400..=172_799 => "ayer".to_owned(),
        172_800..=604_799 => format!("hace {} días", elapsed / 86_400),
        _ => datetime(timestamp),
    }
}

#[cfg(test)]
mod tests {
    use super::relative;

    #[test]
    fn relative_ranges() {
        assert_eq!(relative(1_000, 1_030), "ahora");
        assert_eq!(relative(0, 300), "hace 5 min");
        assert_eq!(relative(0, 7_200), "hace 2 h");
        assert_eq!(relative(0, 90_000), "ayer");
        assert_eq!(relative(0, 3 * 86_400), "hace 3 días");
        assert_eq!(relative(500, 100), "ahora");
    }
}
