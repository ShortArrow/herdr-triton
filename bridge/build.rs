//! Delay-loads the DLLs only serial port enumeration uses, so the resident
//! listener, which never enumerates ports, does not load them (ADR 0012).

fn main() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if msvc {
        for dll in ["setupapi.dll", "cfgmgr32.dll", "advapi32.dll", "bcryptprimitives.dll"] {
            println!("cargo:rustc-link-arg-bin=bridge=/DELAYLOAD:{dll}");
        }
        println!("cargo:rustc-link-arg-bin=bridge=delayimp.lib");
    }
}
