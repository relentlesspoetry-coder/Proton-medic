//! Shared read-only snapshots. Writers continue to use fresh scanner checks.
use crate::{diagnostics, scanner};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};
struct Snapshot<T> { value: Option<(Instant, Arc<T>)> }
impl<T> Snapshot<T> {
    const fn new() -> Self { Self { value: None } }
    fn get(&mut self, ttl: Duration, read: impl FnOnce() -> T) -> Arc<T> {
        if let Some((at, value)) = &self.value {
            if at.elapsed() < ttl { return value.clone(); }
        }
        let value = Arc::new(read());
        self.value = Some((Instant::now(), value.clone()));
        value
    }
}
static STEAM: LazyLock<Mutex<Snapshot<scanner::ScanReport>>> = LazyLock::new(|| Mutex::new(Snapshot::new()));
type Analysis = (Arc<scanner::ScanReport>, Instant, Arc<diagnostics::DiagnosticsReport>);
static ANALYSIS: LazyLock<Mutex<Option<Analysis>>> = LazyLock::new(|| Mutex::new(None));
pub fn steam() -> Arc<scanner::ScanReport> {
    STEAM.lock().unwrap_or_else(|e| e.into_inner()).get(Duration::from_secs(30), scanner::scan)
}
pub fn refresh() -> Arc<scanner::ScanReport> {
    STEAM.lock().unwrap_or_else(|e| e.into_inner()).get(Duration::ZERO, scanner::scan)
}
pub fn analysis(scan: &Arc<scanner::ScanReport>) -> Arc<diagnostics::DiagnosticsReport> {
    let mut cached = ANALYSIS.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((source, at, report)) = &*cached {
        if Arc::ptr_eq(source, scan) && at.elapsed() < Duration::from_secs(2) { return report.clone(); }
    }
    let report = Arc::new(diagnostics::diagnose(scan));
    *cached = Some((scan.clone(), Instant::now(), report.clone()));
    report
}
#[cfg(test)] mod tests {
    use super::*;
    #[test] fn snapshot_reuses_reads_and_explicit_refresh_replaces_them() {
        let mut cache = Snapshot::new();
        let first = cache.get(Duration::from_secs(30), || 1);
        let second = cache.get(Duration::from_secs(30), || panic!("duplicate read"));
        assert!(Arc::ptr_eq(&first, &second));
        let fresh = cache.get(Duration::ZERO, || 2);
        assert_eq!(*fresh, 2); assert!(!Arc::ptr_eq(&first, &fresh));
    }
}
