//! Windows only: embed the app icon and version info (VERSIONINFO) into `mac-word.exe`, so it
//! shows in Explorer, the taskbar, the Start menu and Alt-Tab.
//!
//! On every other target this does nothing. A missing resource compiler is a warning, so a
//! cross-compile from macOS or Linux still links, unless `MACWORD_REQUIRE_WINRES=1` turns it
//! into an error (for release builds).

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../assets/app-icon/mac-word.ico");
    println!("cargo:rerun-if-env-changed=MACWORD_REQUIRE_WINRES");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut res = winresource::WindowsResource::new();
    res.set_icon("../../assets/app-icon/mac-word.ico")
        .set("ProductName", "Mac-Word")
        .set("FileDescription", "Mac-Word word processor")
        .set("CompanyName", "Manik Pruthi")
        .set("LegalCopyright", "Copyright (c) 2026 Manik Pruthi. MIT OR Apache-2.0.")
        .set("OriginalFilename", "mac-word.exe")
        .set("InternalName", "mac-word");
    if let Err(e) = res.compile() {
        if std::env::var_os("MACWORD_REQUIRE_WINRES").is_some() {
            eprintln!("embedding Windows resources failed: {e}");
            std::process::exit(1);
        }
        println!("cargo:warning=mac-word.exe built without icon/version resources: {e}");
    }
}
