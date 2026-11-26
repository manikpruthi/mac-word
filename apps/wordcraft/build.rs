//! Windows only: embed the app icon and version info (VERSIONINFO) into `mac-word.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `MACWORD_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

#[path = "src/build_channel.rs"]
#[allow(dead_code)]
mod build_channel;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=WORDCRAFT_BUILD_CHANNEL");
    println!("cargo:rustc-check-cfg=cfg(wordcraft_nightly_v1)");
    let nightly = match std::env::var("WORDCRAFT_BUILD_CHANNEL").as_deref() {
        Ok("nightly-v1") => {
            println!("cargo:rustc-cfg=wordcraft_nightly_v1");
            true
        }
        // An empty value (an unset CI input) means the default stable build.
        Ok("stable" | "") | Err(std::env::VarError::NotPresent) => false,
        _ => {
            eprintln!("unsupported WORDCRAFT_BUILD_CHANNEL (expected stable or nightly-v1)");
            std::process::exit(1);
        }
    };
    println!("cargo:rerun-if-changed=../../assets/app-icon/mac-word.ico");
    println!("cargo:rerun-if-env-changed=MACWORD_REQUIRE_WINRES");
    println!("cargo:rerun-if-env-changed=WORDCRAFT_REQUIRE_WINRES");
    let built_version = std::env::var("CARGO_PKG_VERSION").unwrap_or_default();
    println!("cargo:rustc-env=WORDCRAFT_BUILT_VERSION={built_version}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let _ = nightly;
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/mac-word.ico")
        .set("ProductName", "Mac-Word")
        .set("FileDescription", "Mac-Word word processor")
        .set("CompanyName", "Manik Pruthi")
        .set("LegalCopyright", "Copyright (c) 2026 Manik Pruthi. MIT OR Apache-2.0.")
        .set("OriginalFilename", "mac-word.exe")
        .set("InternalName", "mac-word");
    if let Err(e) = res.compile() {
        if std::env::var_os("MACWORD_REQUIRE_WINRES").is_some() || std::env::var_os("WORDCRAFT_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=mac-word.exe built without icon/version resources: {e}");
    }
}
