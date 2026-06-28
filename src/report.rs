//! 运行报告与退出码（plan §5.1 / §6.9 / S9 / A10 — M1.15）。
//!
//! 覆盖率（S1）与失败率（S9）独立度量；max-pages 截断标 Partial 且不判失败。

use crate::writer::{Manifest, PageStatus};

/// 一次运行的度量汇总。
#[derive(Debug, Clone)]
pub struct RunReport {
    /// 基准全集（抓取后剔除 Excluded 的最终值）。
    pub baseline_total: usize,
    /// 已入队页数（截断后）。
    pub discovered: usize,
    pub ok: usize,
    pub failed: usize,
    pub excluded: usize,
    /// `ok / baseline_total`（S1）。
    pub coverage: f64,
    /// 是否因 max-pages 截断（§2.1，非失败）。
    pub partial: bool,
    pub uncovered: usize,
    /// `failed / discovered`（S9）。
    pub failure_rate: f64,
    pub warnings: Vec<String>,
}

impl RunReport {
    /// 由 manifest 与入队数构建报告（度量定义见 plan §5.1）。
    pub fn build(manifest: &Manifest, discovered: usize, warnings: Vec<String>) -> Self {
        let (mut ok, mut failed, mut excluded) = (0usize, 0usize, 0usize);
        for r in manifest.pages.values() {
            match r.status {
                PageStatus::Written => ok += 1,
                PageStatus::Failed => failed += 1,
                PageStatus::Excluded => excluded += 1,
                PageStatus::Pending | PageStatus::Fetched => {}
            }
        }
        let baseline_total = manifest.baseline_total.saturating_sub(excluded);
        let coverage = if baseline_total == 0 {
            1.0
        } else {
            ok as f64 / baseline_total as f64
        };
        let failure_rate = if discovered == 0 {
            0.0
        } else {
            failed as f64 / discovered as f64
        };
        RunReport {
            baseline_total,
            discovered,
            ok,
            failed,
            excluded,
            coverage,
            partial: manifest.truncated,
            uncovered: baseline_total.saturating_sub(ok),
            failure_rate,
            warnings,
        }
    }

    /// 是否整次失败：非截断的覆盖率 < 95%，或失败率超阈值（S1×S9 独立；截断 Partial 不算失败）。
    pub fn is_failure(&self, max_failure_rate: f64) -> bool {
        let coverage_fail = !self.partial && self.coverage < 0.95;
        let rate_fail = self.failure_rate > max_failure_rate;
        coverage_fail || rate_fail
    }

    /// 进程退出码。
    pub fn exit_code(&self, max_failure_rate: f64) -> i32 {
        i32::from(self.is_failure(max_failure_rate))
    }

    /// 打印汇总（覆盖率 / Partial 与失败率分列）。
    pub fn print(&self) {
        tracing::info!(
            baseline = self.baseline_total,
            ok = self.ok,
            failed = self.failed,
            excluded = self.excluded,
            coverage = self.coverage,
            failure_rate = self.failure_rate,
            partial = self.partial,
            "run complete"
        );
        if self.partial {
            tracing::warn!(
                uncovered = self.uncovered,
                "partial crawl (max-pages reached): raise --max-pages for full coverage"
            );
        }
        for w in &self.warnings {
            tracing::warn!("{w}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::RuleSet;
    use crate::writer::{Manifest, PageRecord, PageStatus};
    use std::collections::BTreeMap;

    fn rec(status: PageStatus) -> PageRecord {
        PageRecord {
            url: "u".into(),
            rel_path: "p.md".into(),
            status,
            cache: None,
            assets: vec![],
            error: None,
        }
    }

    fn manifest(
        written: usize,
        failed: usize,
        excluded: usize,
        baseline: usize,
        trunc: bool,
    ) -> Manifest {
        let mut pages = BTreeMap::new();
        let mut i = 0;
        for _ in 0..written {
            pages.insert(format!("w{i}"), rec(PageStatus::Written));
            i += 1;
        }
        for _ in 0..failed {
            pages.insert(format!("f{i}"), rec(PageStatus::Failed));
            i += 1;
        }
        for _ in 0..excluded {
            pages.insert(format!("e{i}"), rec(PageStatus::Excluded));
            i += 1;
        }
        Manifest {
            root_url: String::new(),
            prefix: String::new(),
            rules: RuleSet::fallback(),
            baseline_total: baseline,
            truncated: trunc,
            nav_order: vec![],
            pages,
            assets_seen: BTreeMap::new(),
        }
    }

    #[test]
    fn full_coverage_passes() {
        let r = RunReport::build(&manifest(10, 0, 0, 10, false), 10, vec![]);
        assert_eq!(r.ok, 10);
        assert!((r.coverage - 1.0).abs() < 1e-9);
        assert!(!r.is_failure(0.2));
    }

    #[test]
    fn high_failure_rate_fails() {
        let r = RunReport::build(&manifest(5, 5, 0, 10, false), 10, vec![]);
        assert!((r.failure_rate - 0.5).abs() < 1e-9);
        assert!(r.is_failure(0.2));
        assert_eq!(r.exit_code(0.2), 1);
    }

    #[test]
    fn excluded_removed_from_baseline() {
        let r = RunReport::build(&manifest(8, 0, 2, 10, false), 10, vec![]);
        assert_eq!(r.baseline_total, 8);
        assert!((r.coverage - 1.0).abs() < 1e-9);
        assert!(!r.is_failure(0.2));
    }

    #[test]
    fn partial_truncation_not_failure() {
        let r = RunReport::build(&manifest(10, 0, 0, 100, true), 10, vec![]);
        assert!(r.partial);
        assert!(r.coverage < 0.95);
        assert!(!r.is_failure(0.2));
    }
}
