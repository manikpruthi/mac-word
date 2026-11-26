//! WordCraft's contributor metadata: every contributor (GitHub login always; display and real names only
//! with recorded consent) and the AI models that helped. The tables are baked into the binary by
//! build.rs from contributors/contributors.json; nothing is
//! read at run time.

use std::borrow::Cow;

use craft::ui::patterns::form_dialogs::{AboutInfo, Contributor as ContributorCredit, Credits, ModelCredit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contributor {
    pub login: &'static str,
    pub display_name: Option<&'static str>,
    pub real_name: Option<&'static str>,
    pub prs: u64,
    pub commits: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
    pub binary_added: u64,
    pub binary_deleted: u64,
    /// ISO 8601 UTC; empty for someone credited only through a merged PR.
    pub first_commit: &'static str,
    pub last_commit: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Model {
    pub company: &'static str,
    pub model: &'static str,
    pub version: &'static str,
    pub commits: u64,
    pub lines_added: u64,
    pub lines_deleted: u64,
}

include!(concat!(env!("OUT_DIR"), "/credits.rs"));

/// The baked contributor and model tables as the About dialog's credits. A name is only present where
/// build.rs recorded consent for it; the profile link is the contributor's GitHub page.
pub fn credits() -> Credits {
    let contributors: Vec<ContributorCredit> = CONTRIBUTORS
        .iter()
        .map(|c| {
            let mut credit = ContributorCredit::new(c.login)
                .profile(Cow::Owned(format!("https://github.com/{}", c.login)))
                .counts(c.prs, c.commits)
                .lines(c.lines_added, c.lines_deleted)
                .binary(c.binary_added, c.binary_deleted)
                .active(c.first_commit, c.last_commit);
            if let Some(name) = c.display_name {
                credit = credit.display_name(name);
            }
            if let Some(name) = c.real_name {
                credit = credit.real_name(name);
            }
            credit
        })
        .collect();
    let models: Vec<ModelCredit> =
        MODELS.iter().map(|m| ModelCredit::new(m.company, m.model, m.version).work(m.commits, m.lines_added, m.lines_deleted)).collect();
    Credits::new(contributors, models, TOTAL_COMMITS)
}

/// The app name, description, version, build and licence shown in About.
pub fn about_info() -> AboutInfo {
    let identity = crate::os_window::identity();
    let commit = match identity {
        crate::os_window::LaunchIdentity::Stable => option_env!("MACWORD_BUILD_SHA"),
        crate::os_window::LaunchIdentity::NightlyV1 => option_env!("WORDCRAFT_BUILD_SHA"),
    };
    about_info_for(identity, commit)
}

/// The About tab's three lines for `identity`: (name, blurb, version line). A Nightly window's
/// are English only: they are not one of the stable catalogs' strings.
fn about_text(identity: crate::os_window::LaunchIdentity, commit: Option<&str>) -> (String, String, String) {
    match identity {
        crate::os_window::LaunchIdentity::NightlyV1 => {
            let version = option_env!("WORDCRAFT_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
            let commit = commit.filter(|value| !value.is_empty()).unwrap_or("unavailable");
            (
                crate::os_window::name_for(identity).to_string(),
                "From Manik Pruthi. A native, Microsoft Word-compatible word processor for macOS.\nBuilt in Rust with a Metal-rendered interface. No update checks. No telemetry.".to_string(),
                format!("{version}\nCommit: {commit}"),
            )
        }
        crate::os_window::LaunchIdentity::Stable => {
            let build = commit.map(|s| s.get(..8).unwrap_or(s)).unwrap_or(tl!("development build"));
            // The dialog's own "Version {version}" words the line, so this is the number and the build only.
            let version = crate::i18n::fmt(tl!("{version} ({build})"), &[("version", env!("CARGO_PKG_VERSION")), ("build", build)]);
            (
                tl!("Mac-Word").to_string(),
                tl!("From Manik Pruthi. A native, Microsoft Word-compatible word processor for macOS.\nBuilt in Rust with a Metal-rendered interface. No update checks. No telemetry. Your documents stay on your Mac.")
                    .to_string(),
                version.to_string(),
            )
        }
    }
}

fn about_info_for(identity: crate::os_window::LaunchIdentity, commit: Option<&str>) -> AboutInfo {
    let (name, blurb, version) = about_text(identity, commit);
    if identity == crate::os_window::LaunchIdentity::NightlyV1 {
        return AboutInfo::new(name, blurb, version)
            .built_with("MIT OR Apache-2.0. Copyright (c) 2026 Manik Pruthi. Based on WordCraft, copyright (c) 2026 ArtCraft Team and the WordCraft contributors.".to_string());
    }
    AboutInfo::new(name, blurb, version).built_with(
        tl!(
            "MIT OR Apache-2.0. Copyright (c) 2026 Manik Pruthi. Based on WordCraft, copyright (c) 2026 ArtCraft Team and the WordCraft contributors."
        )
        .to_string(),
    )
}

/// A Nightly window's About names its channel, keeps the full feed version and commit, invents
/// and reports when the commit is unavailable.
#[cfg(test)]
mod nightly_about_tests {
    use crate::os_window::LaunchIdentity;

    const COMMIT: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn nightly_about_names_the_channel_and_keeps_the_full_version_and_commit() {
        let (name, blurb, version) = super::about_text(LaunchIdentity::NightlyV1, Some(COMMIT));
        assert_eq!(name, "Mac-Word Nightly");
        assert!(blurb.contains("word processor"));
        let package = option_env!("WORDCRAFT_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
        assert_eq!(version, format!("{package}\nCommit: {COMMIT}"));
    }

    #[test]
    fn nightly_about_invents_no_provenance_when_the_commit_is_missing() {
        for commit in [None, Some("")] {
            let (_, _, version) = super::about_text(LaunchIdentity::NightlyV1, commit);
            assert!(version.ends_with("\nCommit: unavailable"), "{version}");
        }
    }

    #[test]
    fn stable_about_names_mac_word_and_has_no_report_link() {
        let stable = super::about_info_for(LaunchIdentity::Stable, Some(COMMIT));
        assert!(stable.links.is_empty());
        let (name, _, _) = super::about_text(LaunchIdentity::Stable, Some(COMMIT));
        assert_eq!(name, "Mac-Word");
    }
}
