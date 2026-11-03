#[cfg(not(target_arch = "wasm32"))]
use super::Ordering;
use super::face::{FontBytes, FontFace};
#[cfg(not(target_arch = "wasm32"))]
use super::names::{CatalogEntry, file_face_names, style_italic, style_weight};
use super::names::{LocalName, MAX_FAMILY_NAME, MAX_FILE_FACES, enumerate_faces, make_face, name_language_matches};
use super::{Arc, FontDb, HashMap};

/// Version of the on-disk catalog. Bump it when [`CatalogEntry`] changes in a way older files
/// cannot describe.
#[cfg(not(target_arch = "wasm32"))]
const CATALOG_CACHE_VERSION: u32 = 1;

/// The installed-font catalog between runs: every scanned folder and font file with the timestamp
/// it had, so a changed, added or removed font invalidates the whole cache.
#[cfg(not(target_arch = "wasm32"))]
#[derive(serde::Serialize, serde::Deserialize)]
struct CatalogCache {
    version: u32,
    roots: Vec<std::path::PathBuf>,
    dirs: Vec<(std::path::PathBuf, u64, u32)>,
    files: Vec<(std::path::PathBuf, u64, u64, u32)>,
    entries: Vec<CatalogEntry>,
}

/// A modification time as whole seconds and nanoseconds since the epoch.
#[cfg(not(target_arch = "wasm32"))]
fn stamp(time: std::time::SystemTime) -> Option<(u64, u32)> {
    time.duration_since(std::time::UNIX_EPOCH).ok().map(|d| (d.as_secs(), d.subsec_nanos()))
}

/// A file's length and modification time, or `None` when it can't be read.
#[cfg(not(target_arch = "wasm32"))]
fn file_stamp(path: &std::path::Path) -> Option<(u64, u64, u32)> {
    let m = std::fs::metadata(path).ok()?;
    if !m.is_file() {
        return None;
    }
    let (secs, nanos) = stamp(m.modified().ok()?)?;
    Some((m.len(), secs, nanos))
}

/// The faces of every font file, parsed in parallel (the first scan reads hundreds of files).
#[cfg(not(target_arch = "wasm32"))]
fn catalog_files(paths: &[std::path::PathBuf]) -> Vec<CatalogEntry> {
    let parse = |path: &std::path::PathBuf| -> Vec<CatalogEntry> {
        file_face_names(path)
            .into_iter()
            .map(|f| CatalogEntry {
                family: f.family,
                style: f.style,
                legacy: f.legacy,
                full_name: f.full_name,
                postscript: f.postscript,
                local: f.local,
                east_asian: f.east_asian,
                path: path.clone(),
            })
            .collect()
    };
    let workers = std::thread::available_parallelism().map_or(1, std::num::NonZero::get).min(8).min(paths.len().max(1));
    if workers <= 1 || paths.len() < 16 {
        return paths.iter().flat_map(parse).collect();
    }
    let chunk = paths.len().div_ceil(workers);
    let mut found = Vec::new();
    std::thread::scope(|scope| {
        let handles: Vec<_> = paths.chunks(chunk).map(|part| scope.spawn(move || part.iter().flat_map(parse).collect::<Vec<_>>())).collect();
        for handle in handles {
            // A worker that panicked on one damaged file leaves only its own files out; the rest
            // of the catalog still loads.
            if let Ok(faces) = handle.join() {
                found.extend(faces);
            }
        }
    });
    found
}

impl FontDb {
    /// Family names available (loaded plus cataloged system fonts), sorted and deduplicated. Like
    /// Word's font list, this holds each face's legacy family name too ("Roboto Light", "Inter
    /// SemiBold"), which [`FontDb::face`] resolves to that face.
    pub fn families(&self) -> Vec<String> {
        let mut v: Vec<String> = self.read_faces().iter().flat_map(|f| [f.family.clone(), f.legacy_family.clone()]).collect();
        #[cfg(not(target_arch = "wasm32"))]
        v.extend(self.read_catalog().iter().flat_map(|c| [c.family.clone(), c.legacy.clone()]));
        v.sort_by_key(|a| a.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    /// The installed Chinese, Japanese and Korean families (English names), for the Font dialog's
    /// Asian text font list.
    pub fn east_asian_families(&self) -> Vec<String> {
        let mut v: Vec<String> = self.read_faces().iter().filter(|f| f.is_east_asian()).map(|f| f.family.clone()).collect();
        #[cfg(not(target_arch = "wasm32"))]
        v.extend(self.read_catalog().iter().filter(|c| c.east_asian).map(|c| c.family.clone()));
        v.sort_by_key(|a| a.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    /// Every family's name in the interface language `lang` (`zh-hans`, `zh-hant`, `ja`…), as
    /// (English name, localized name) pairs: SimSun is 宋体 in Simplified Chinese. Families without
    /// a name in that language are left out.
    pub fn localized_names(&self, lang: &str) -> Vec<(String, String)> {
        let mut v: Vec<(String, String)> = Vec::new();
        let mut add = |names: &[LocalName]| {
            for n in names.iter().filter(|n| name_language_matches(lang, &n.lang)) {
                if !v.iter().any(|(e, _)| e.eq_ignore_ascii_case(&n.english)) {
                    v.push((n.english.clone(), n.name.clone()));
                }
            }
        };
        for f in self.read_faces().iter() {
            add(&f.local_names);
        }
        #[cfg(not(target_arch = "wasm32"))]
        for c in self.read_catalog().iter() {
            add(&c.local);
        }
        v
    }

    /// [`FontDb::families`] as the font list shows them in the interface language `lang`: each
    /// family that has a name in that language by that name (宋体, 微软雅黑 in Simplified Chinese),
    /// the others by their English names. Lookups accept either name.
    pub fn families_in(&self, lang: &str) -> Vec<String> {
        let local: HashMap<String, String> = self.localized_names(lang).into_iter().map(|(e, l)| (e.to_lowercase(), l)).collect();
        let mut v: Vec<String> = self.families().into_iter().map(|f| local.get(&f.to_lowercase()).cloned().unwrap_or(f)).collect();
        v.sort_by_key(|a| a.to_lowercase());
        v.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        v
    }

    /// The English family a localized family name stands for (宋体 → SimSun), when no face has
    /// `family` as its English name. `None` for names no font has.
    pub fn english_family(&self, family: &str) -> Option<String> {
        let hit = |names: &[LocalName]| names.iter().find(|n| n.name.eq_ignore_ascii_case(family)).map(|n| n.english.clone());
        if let Some(e) = self.read_faces().iter().find_map(|f| hit(&f.local_names)) {
            return Some(e);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(e) = self.read_catalog().iter().find_map(|c| hit(&c.local)) {
            return Some(e);
        }
        None
    }

    /// Style names available for `family` (Regular first, then by weight). For a legacy family
    /// name ("Roboto Light"), the styles of the faces it names ("Light", "Light Italic"); for a
    /// localized name (宋体), those of the family it names.
    pub fn styles(&self, family: &str) -> Vec<String> {
        let v = self.styles_named(family);
        match self.english_family(family) {
            Some(en) if v.is_empty() => self.styles_named(&en),
            _ => v,
        }
    }

    fn styles_named(&self, family: &str) -> Vec<String> {
        let mut v: Vec<(bool, f32, String)> = Vec::new();
        for legacy in [false, true] {
            let named = |typographic: &str, legacy_name: &str| (if legacy { legacy_name } else { typographic }).eq_ignore_ascii_case(family);
            v.extend(
                self.read_faces()
                    .iter()
                    .filter(|f| named(&f.family, &f.legacy_family) && !f.style.contains('{'))
                    .map(|f| (f.italic, f.weight, f.style.clone())),
            );
            #[cfg(not(target_arch = "wasm32"))]
            for c in self.read_catalog().iter() {
                if named(&c.family, &c.legacy) && !v.iter().any(|(_, _, s)| s.eq_ignore_ascii_case(&c.style)) {
                    v.push((style_italic(&c.style), style_weight(&c.style), c.style.clone()));
                }
            }
            if !v.is_empty() {
                break;
            }
        }
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)));
        v.dedup_by(|a, b| a.2 == b.2);
        v.into_iter().map(|t| t.2).collect()
    }

    /// Add a user font (TTF/OTF/TTC bytes). Returns the number of faces added (0 if unparseable or
    /// every face was already present).
    pub fn add_font(&self, bytes: Vec<u8>) -> usize {
        self.add_fonts(vec![(bytes, None)])
    }

    /// Add the faces of several font files at once: lookups see all of them or none, never a
    /// family with only some of its styles (which would resolve, and be cached as, the wrong
    /// face). A file given a `name` its faces don't go by (a document's font table's, see
    /// [`FontDb::add_document_fonts`]) has them go by that name instead. Faces already present
    /// (same family and style) are skipped. Returns how many were added.
    pub(super) fn add_fonts(&self, files: Vec<(Vec<u8>, Option<String>)>) -> usize {
        let mut new = Vec::new();
        for (bytes, name) in files {
            let data = Arc::new(bytes);
            let name: Option<String> = name.map(|n| n.trim().chars().take(MAX_FAMILY_NAME).collect()).filter(|n: &String| !n.is_empty());
            for f in enumerate_faces(&data, MAX_FILE_FACES) {
                if let Some(mut face) = make_face(FontBytes::Owned(data.clone()), f.index, f.family, f.style, f.legacy, f.coords) {
                    face.local_names = f.local;
                    if let Some(n) = name.as_deref().filter(|n| !face.named(n) && !face.local_names.iter().any(|l| l.name.eq_ignore_ascii_case(n))) {
                        face.family = n.to_string();
                        face.legacy_family = n.to_string();
                    }
                    new.push(Arc::new(face));
                }
            }
        }
        let mut faces = self.faces.write().unwrap_or_else(std::sync::PoisonError::into_inner);
        // Indexed: a document's fonts may add hundreds of faces to thousands (#3716).
        let key = |f: &FontFace| (f.family.to_ascii_lowercase(), f.style.to_ascii_lowercase());
        let mut present: std::collections::HashSet<_> = faces.iter().map(|g| key(g)).collect();
        let mut added = 0;
        for f in new {
            if present.insert(key(&f)) {
                faces.push(f);
                added += 1;
            }
        }
        added
    }

    /// Catalog the installed fonts unless that has been done: the first caller scans the font
    /// folders, any other waits for that scan to finish.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn ensure_catalog(&self) {
        self.cataloged.get_or_init(|| {
            self.scan_font_dirs();
        });
    }

    /// Catalog the installed fonts on a background thread, so the first lookup by family name
    /// (opening a file, the font menus) doesn't wait for the scan. A no-op once they are
    /// cataloged, and on wasm.
    pub fn scan_in_background(&'static self) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.cataloged.get().is_none() {
            // A failed spawn leaves the scan to the first lookup that needs it.
            let _ = std::thread::Builder::new().name("font-scan".into()).spawn(move || self.ensure_catalog());
        }
    }

    /// Scan the font folders again (fonts installed or removed since), cataloging the faces
    /// found (native only). Font data is loaded when a cataloged family is first resolved.
    /// Returns the number of faces cataloged.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_system_fonts(&self) -> usize {
        // The first scan, or a rescan once it is done (never both at once).
        let mut first = None;
        self.cataloged.get_or_init(|| first = Some(self.scan_font_dirs()));
        first.unwrap_or_else(|| self.scan_font_dirs())
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn scan_font_dirs(&self) -> usize {
        if let Some(n) = self.load_catalog_cache() {
            log::debug!("loaded the system font catalog from its cache: {n} faces");
            return n;
        }
        let mut files = Vec::new();
        let mut dirs = Vec::new();
        let mut stack = self.font_dirs.clone();
        // Each folder once, however links lead back to it.
        let mut visited = std::collections::HashSet::new();
        while let Some(d) = stack.pop() {
            if !visited.insert(std::fs::canonicalize(&d).unwrap_or_else(|_| d.clone())) {
                continue;
            }
            if let Ok(m) = std::fs::metadata(&d)
                && let Some((secs, nanos)) = m.modified().ok().and_then(stamp)
            {
                dirs.push((d.clone(), secs, nanos));
            }
            let Ok(rd) = std::fs::read_dir(&d) else { continue };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let ext = p.extension().and_then(std::ffi::OsStr::to_str).map(str::to_ascii_lowercase);
                if matches!(ext.as_deref(), Some("ttf" | "otf" | "ttc" | "otc")) {
                    files.push(p);
                }
            }
        }
        files.sort();
        files.dedup();
        let found = catalog_files(&files);
        let n = found.len();
        log::debug!("cataloged {n} system font faces from {} files", files.len());
        self.save_catalog_cache(&dirs, &files, &found);
        *self.catalog.write().unwrap_or_else(std::sync::PoisonError::into_inner) = found;
        self.catalog_gen.fetch_add(1, Ordering::AcqRel);
        n
    }

    /// The cached catalog, when every folder and file it names still has the same timestamp.
    #[cfg(not(target_arch = "wasm32"))]
    fn load_catalog_cache(&self) -> Option<usize> {
        let path = self.catalog_cache.lock().ok()?.clone()?;
        let bytes = std::fs::read(&path).ok()?;
        let cache: CatalogCache = serde_json::from_slice(&bytes).ok()?;
        if cache.version != CATALOG_CACHE_VERSION || cache.roots != self.font_dirs {
            return None;
        }
        for (dir, secs, nanos) in &cache.dirs {
            let m = std::fs::metadata(dir).ok()?;
            if !m.is_dir() || stamp(m.modified().ok()?) != Some((*secs, *nanos)) {
                return None;
            }
        }
        for (file, len, secs, nanos) in &cache.files {
            if file_stamp(file) != Some((*len, *secs, *nanos)) {
                return None;
            }
        }
        let n = cache.entries.len();
        *self.catalog.write().unwrap_or_else(std::sync::PoisonError::into_inner) = cache.entries;
        self.catalog_gen.fetch_add(1, Ordering::AcqRel);
        Some(n)
    }

    /// Store the catalog beside the preferences, atomically, so a failed write leaves the previous
    /// cache in place. A cache that can't be written only costs the next run a scan.
    #[cfg(not(target_arch = "wasm32"))]
    fn save_catalog_cache(&self, dirs: &[(std::path::PathBuf, u64, u32)], files: &[std::path::PathBuf], entries: &[CatalogEntry]) {
        let Some(path) = self.catalog_cache.lock().ok().and_then(|p| p.clone()) else { return };
        let cache = CatalogCache {
            version: CATALOG_CACHE_VERSION,
            roots: self.font_dirs.clone(),
            dirs: dirs.to_vec(),
            files: files.iter().filter_map(|f| file_stamp(f).map(|(len, secs, nanos)| (f.clone(), len, secs, nanos))).collect(),
            entries: entries.to_vec(),
        };
        let Ok(bytes) = serde_json::to_vec(&cache) else { return };
        let Some(parent) = path.parent() else { return };
        if std::fs::create_dir_all(parent).is_err() {
            return;
        }
        let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
        if std::fs::write(&tmp, &bytes).is_ok() && std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    /// Load the files of the installed `family`: the whole typographic family, also when asked for
    /// by a legacy name ("Roboto Light" loads Roboto), so a family never loads with only some of
    /// its styles. Returns whether any face was added.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn load_cataloged(&self, family: &str) -> bool {
        let _loading = self.loading.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        // Another lookup may have loaded it while this one waited.
        if self.is_loaded(family) {
            return true;
        }
        let paths = self.family_paths(family);
        let files = paths.iter().filter_map(|p| std::fs::read(p).ok()).map(|b| (b, None)).collect();
        let added = self.add_fonts(files) > 0;
        // The fallback searches' index: these files are loaded fonts now.
        let mut index = self.coverage.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for file in index.files.iter_mut().filter(|f| paths.contains(&f.path)) {
            file.used = true;
        }
        index.loaded.extend(paths);
        added
    }

    /// The cataloged files holding `family`'s faces (by typographic, legacy or full name), sorted.
    #[cfg(not(target_arch = "wasm32"))]
    pub(super) fn family_paths(&self, family: &str) -> Vec<std::path::PathBuf> {
        let cat = self.read_catalog();
        let families: Vec<&str> = cat
            .iter()
            .filter(|c| {
                c.family.eq_ignore_ascii_case(family)
                    || c.legacy.eq_ignore_ascii_case(family)
                    || c.full_name.as_ref().is_some_and(|n| n.eq_ignore_ascii_case(family))
                    || c.postscript.as_ref().is_some_and(|n| n.eq_ignore_ascii_case(family))
            })
            .map(|c| c.family.as_str())
            .collect();
        let mut p: Vec<_> = cat.iter().filter(|c| families.iter().any(|f| c.family.eq_ignore_ascii_case(f))).map(|c| c.path.clone()).collect();
        p.sort();
        p.dedup();
        p
    }
}
