//! Which process owns the window that shows herdr (ADR 0015).

use std::collections::BTreeSet;

use trident::raise::{host, Process};

fn process(pid: u32, parent: u32, name: &str) -> Process {
    Process {
        pid,
        parent,
        name: name.into(),
    }
}

fn windowed(pids: &[u32]) -> BTreeSet<u32> {
    pids.iter().copied().collect()
}

#[test]
fn in_wezterm_it_passes_the_shell_and_finds_the_terminal() {
    let processes = [
        process(5, 1, "wezterm-gui.exe"),
        process(10, 5, "pwsh.exe"),
        process(20, 10, "herdr.exe"),
        process(30, 20, "herdr.exe"),
    ];
    assert_eq!(host(&processes, &windowed(&[5])), Some(5));
}

#[test]
fn in_a_console_window_the_shell_owns_it() {
    let processes = [process(10, 1, "pwsh.exe"), process(20, 10, "herdr.exe")];
    assert_eq!(host(&processes, &windowed(&[10])), Some(10));
}

#[test]
fn the_name_is_matched_ignoring_case() {
    let processes = [
        process(5, 1, "WezTerm-GUI.exe"),
        process(20, 5, "HERDR.EXE"),
    ];
    assert_eq!(host(&processes, &windowed(&[5])), Some(5));
}

#[test]
fn without_herdr_there_is_no_host() {
    let processes = [process(5, 1, "wezterm-gui.exe"), process(10, 5, "pwsh.exe")];
    assert_eq!(host(&processes, &windowed(&[5])), None);
}

#[test]
fn a_server_whose_client_has_gone_has_no_host() {
    let processes = [
        process(5, 1, "wezterm-gui.exe"),
        process(30, 20, "herdr.exe"),
    ];
    assert_eq!(host(&processes, &windowed(&[5])), None);
}

#[test]
fn an_ancestry_without_a_window_has_no_host() {
    let processes = [process(10, 1, "pwsh.exe"), process(20, 10, "herdr.exe")];
    assert_eq!(host(&processes, &windowed(&[])), None);
}

#[test]
fn a_parent_loop_ends_without_a_host() {
    let processes = [process(10, 20, "pwsh.exe"), process(20, 10, "herdr.exe")];
    assert_eq!(host(&processes, &windowed(&[])), None);
}

#[test]
fn a_second_client_is_tried_when_the_first_has_no_window() {
    let processes = [
        process(20, 2, "herdr.exe"),
        process(5, 1, "wezterm-gui.exe"),
        process(21, 5, "herdr.exe"),
    ];
    assert_eq!(host(&processes, &windowed(&[5])), Some(5));
}
