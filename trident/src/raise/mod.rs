//! Finding the window that shows herdr (ADR 0015).

use std::collections::{BTreeMap, BTreeSet};

/// One running process, as a process snapshot lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Process {
    pub pid: u32,
    pub parent: u32,
    /// The executable's file name, such as `herdr.exe`.
    pub name: String,
}

const HERDR: &str = "herdr.exe";

/// The process that owns the window showing herdr: from each herdr client,
/// in `processes` order, the nearest ancestor, itself included, whose pid is
/// in `windowed`. A client is a `herdr.exe` whose parent is not one.
pub fn host(processes: &[Process], windowed: &BTreeSet<u32>) -> Option<u32> {
    let by_pid: BTreeMap<u32, &Process> = processes.iter().map(|p| (p.pid, p)).collect();
    let is_herdr = |pid: u32| {
        by_pid
            .get(&pid)
            .is_some_and(|p| p.name.eq_ignore_ascii_case(HERDR))
    };
    processes
        .iter()
        .filter(|p| p.name.eq_ignore_ascii_case(HERDR) && !is_herdr(p.parent))
        .find_map(|client| owner(client.pid, &by_pid, windowed))
}

/// The nearest of `pid` and its ancestors in `windowed`, visiting each
/// process at most once.
fn owner(pid: u32, by_pid: &BTreeMap<u32, &Process>, windowed: &BTreeSet<u32>) -> Option<u32> {
    let mut seen = BTreeSet::new();
    let mut current = by_pid.get(&pid)?;
    loop {
        if windowed.contains(&current.pid) {
            return Some(current.pid);
        }
        if !seen.insert(current.pid) {
            return None;
        }
        current = by_pid.get(&current.parent)?;
    }
}
