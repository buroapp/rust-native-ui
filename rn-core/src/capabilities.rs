//! Backend capability declaration and completeness checking.
//!
//! Constitution VI requires the renderer to be a thin, per-platform consumer of
//! a changeset. That only holds together if every backend actually handles every
//! component kind. This module is how a backend proves it does, and how the test
//! suite catches the case where it silently does not.
//!
//! Two separate guarantees, deliberately:
//!
//! 1. **Compile time** — a backend matches exhaustively on [`ComponentKind`] with
//!    no `_` catch-all. A new variant is a build error on every backend until it
//!    is implemented. This is the real guard.
//! 2. **Test time** — [`coverage_report`] compares a backend's declared
//!    capabilities against [`ComponentKind::ALL`]. This catches the subtler
//!    failure where a kind is matched but deliberately no-ops.
//!
//! Neither guarantee is sufficient alone. The first cannot see a no-op arm; the
//! second cannot run before the code compiles.

use crate::ComponentKind;

/// What a single backend reports about its own implementation status.
#[derive(Debug, Clone, Default)]
pub struct CapabilityReport {
    /// Kinds the backend claims to implement.
    pub implemented: Vec<ComponentKind>,
}

impl CapabilityReport {
    /// Build a report from a backend's declared capability slice.
    #[must_use]
    pub fn new(implemented: &[ComponentKind]) -> Self {
        Self {
            implemented: implemented.to_vec(),
        }
    }

    /// Kinds present in [`ComponentKind::ALL`] but absent from this report.
    ///
    /// An empty result means the backend claims full coverage.
    #[must_use]
    pub fn missing(&self) -> Vec<ComponentKind> {
        ComponentKind::ALL
            .iter()
            .copied()
            .filter(|k| !self.implemented.contains(k))
            .collect()
    }

    /// Kinds the backend declares that are not valid component kinds.
    ///
    /// Should always be empty. A non-empty result means the capability list has
    /// drifted from the enum — usually a renamed or removed variant.
    #[must_use]
    pub fn unknown(&self) -> Vec<ComponentKind> {
        self.implemented
            .iter()
            .copied()
            .filter(|k| !ComponentKind::ALL.contains(k))
            .collect()
    }

    /// Kinds listed more than once.
    ///
    /// Duplicates are harmless at runtime but indicate a copy-paste error in a
    /// hand-maintained capability table, which is worth surfacing.
    #[must_use]
    pub fn duplicates(&self) -> Vec<ComponentKind> {
        let mut seen: Vec<ComponentKind> = Vec::new();
        let mut dups: Vec<ComponentKind> = Vec::new();
        for kind in &self.implemented {
            if seen.contains(kind) {
                if !dups.contains(kind) {
                    dups.push(*kind);
                }
            } else {
                seen.push(*kind);
            }
        }
        dups
    }

    /// True when the backend claims to implement every component kind exactly once.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.missing().is_empty() && self.unknown().is_empty() && self.duplicates().is_empty()
    }
}

/// Human-readable description of everything wrong with a capability report.
///
/// Returns `None` when the report is complete, so callers can write
/// `assert!(coverage_report(..).is_none())` and get the failure detail in the
/// assertion message via `.expect(..)` in tests.
#[must_use]
pub fn coverage_report(report: &CapabilityReport) -> Option<String> {
    if report.is_complete() {
        return None;
    }
    let missing = report.missing();
    let unknown = report.unknown();
    let duplicates = report.duplicates();
    let mut out = String::new();
    if !missing.is_empty() {
        out.push_str(&format!("missing {} kind(s): {:?}", missing.len(), missing));
    }
    if !unknown.is_empty() {
        if !out.is_empty() {
            out.push_str("; ");
        }
        out.push_str(&format!("unknown {} kind(s): {:?}", unknown.len(), unknown));
    }
    if !duplicates.is_empty() {
        if !out.is_empty() {
            out.push_str("; ");
        }
        out.push_str(&format!(
            "duplicate {} kind(s): {:?}",
            duplicates.len(),
            duplicates
        ));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_list_matches_declared_count() {
        assert_eq!(
            ComponentKind::ALL.len(),
            ComponentKind::COUNT,
            "ComponentKind::ALL has {} entries but COUNT says {} — update both",
            ComponentKind::ALL.len(),
            ComponentKind::COUNT
        );
    }

    #[test]
    fn all_list_has_no_duplicates() {
        let report = CapabilityReport::new(ComponentKind::ALL);
        assert_eq!(
            report.duplicates(),
            Vec::<ComponentKind>::new(),
            "ComponentKind::ALL contains duplicates"
        );
    }

    #[test]
    fn full_coverage_is_complete() {
        let report = CapabilityReport::new(ComponentKind::ALL);
        assert!(report.is_complete());
        assert!(coverage_report(&report).is_none());
    }

    #[test]
    fn missing_kind_is_reported() {
        let partial = &ComponentKind::ALL[..ComponentKind::ALL.len() - 1];
        let report = CapabilityReport::new(partial);
        assert!(!report.is_complete());
        assert_eq!(report.missing().len(), 1);
        let msg = coverage_report(&report).unwrap_or_default();
        assert!(msg.contains("missing 1 kind"));
    }

    #[test]
    fn duplicate_kind_is_reported() {
        let mut kinds = ComponentKind::ALL.to_vec();
        kinds.push(ComponentKind::Button);
        let report = CapabilityReport::new(&kinds);
        assert!(!report.is_complete());
        assert_eq!(report.duplicates(), vec![ComponentKind::Button]);
    }

    #[test]
    fn empty_report_lists_everything_missing() {
        let report = CapabilityReport::default();
        assert_eq!(report.missing().len(), ComponentKind::COUNT);
    }
}
