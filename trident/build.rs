//! Delay-loads the DLLs only serial port enumeration and `trident raise` use,
//! so the resident listener, which does neither, does not load them
//! (ADR 0012, ADR 0015).

fn main() {
    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if msvc {
        for dll in [
            "setupapi.dll",
            "cfgmgr32.dll",
            "advapi32.dll",
            "bcryptprimitives.dll",
            "user32.dll",
        ] {
            println!("cargo:rustc-link-arg-bin=trident=/DELAYLOAD:{dll}");
        }
        println!("cargo:rustc-link-arg-bin=trident=delayimp.lib");
    }
}
