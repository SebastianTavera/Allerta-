//! Estadísticas agregadas para el panel de resumen.

use std::collections::HashMap;

use crate::domain::{Category, Coded, Report, Severity, Status};

const HOTSPOT_LIMIT: usize = 5;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub total: usize,
    pub open: usize,
    pub in_review: usize,
    pub resolved: usize,
    pub critical_active: usize,
    pub by_category: Vec<(Category, usize)>,
    pub by_severity: Vec<(Severity, usize)>,
    /// Ubicaciones con más reportes activos.
    pub hotspots: Vec<(String, usize)>,
}

impl Summary {
    pub fn from_reports(reports: &[Report]) -> Self {
        let count = |predicate: &dyn Fn(&Report) -> bool| reports.iter().filter(|r| predicate(r)).count();

        Self {
            total: reports.len(),
            open: count(&|r| r.status == Status::Open),
            in_review: count(&|r| r.status == Status::InReview),
            resolved: count(&|r| r.status == Status::Resolved),
            critical_active: count(&|r| r.is_active() && r.severity == Severity::Critical),
            by_category: Category::ALL
                .iter()
                .map(|&category| (category, count(&|r| r.category == category)))
                .collect(),
            by_severity: Severity::ALL
                .iter()
                .map(|&severity| (severity, count(&|r| r.severity == severity)))
                .collect(),
            hotspots: hotspots(reports),
        }
    }

    pub fn active(&self) -> usize {
        self.open + self.in_review
    }
}

fn hotspots(reports: &[Report]) -> Vec<(String, usize)> {
    // Agrupa sin distinguir mayúsculas; conserva la primera forma escrita.
    let mut groups: HashMap<String, (String, usize)> = HashMap::new();
    for report in reports.iter().filter(|r| r.is_active()) {
        let key = report.location.trim().to_lowercase();
        groups
            .entry(key)
            .or_insert_with(|| (report.location.trim().to_owned(), 0))
            .1 += 1;
    }
    let mut ranked: Vec<(String, usize)> = groups.into_values().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    ranked.truncate(HOTSPOT_LIMIT);
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ReportId;

    fn report(id: i64, location: &str, status: Status, severity: Severity) -> Report {
        Report {
            id: ReportId(id),
            category: Category::UnsafeZone,
            severity,
            status,
            title: "t".into(),
            location: location.into(),
            description: "d".into(),
            reporter: None,
            created_at: 0,
            notified: true,
        }
    }

    #[test]
    fn aggregates_counts_and_hotspots() {
        let reports = vec![
            report(1, "Bloque A", Status::Open, Severity::Critical),
            report(2, "bloque a", Status::InReview, Severity::Low),
            report(3, "Cafetería", Status::Open, Severity::High),
            report(4, "Cafetería", Status::Resolved, Severity::Critical),
        ];
        let summary = Summary::from_reports(&reports);
        assert_eq!(summary.total, 4);
        assert_eq!(summary.active(), 3);
        assert_eq!(summary.resolved, 1);
        assert_eq!(summary.critical_active, 1);
        assert_eq!(summary.hotspots[0], ("Bloque A".to_owned(), 2));
        assert_eq!(summary.hotspots[1], ("Cafetería".to_owned(), 1));
        assert_eq!(summary.by_category[0], (Category::UnsafeZone, 4));
    }
}
