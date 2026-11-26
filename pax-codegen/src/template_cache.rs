//! A disposable host-build cache, not a cartridge or hot-reload payload.
use pax_manifest::parsing::{LoweredTemplate, TemplateNodeParseContext};
use pax_manifest::TypeId;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

type Key = [u8; 32];

pub(super) fn directory() -> Option<PathBuf> {
    if std::env::var("PAX_BUILD_DESIGNTIME").as_deref() != Ok("1") {
        return None;
    }
    let dir = std::env::var_os("PAX_DIR").filter(|dir| !dir.is_empty())?;
    Some(PathBuf::from(dir).join("dev/template-cache"))
}

// Identify the linked host parser, including local, unversioned compiler edits.
// In rustc this is the proc-macro library, not the rustc executable. If its
// identity is unavailable, build normally rather than guessing compatibility.
#[cfg(unix)]
fn compiler_key() -> Option<Key> {
    use std::{ffi::CStr, os::unix::ffi::OsStrExt, sync::OnceLock};
    static KEY: OnceLock<Option<Key>> = OnceLock::new();
    *KEY.get_or_init(|| {
        let mut info = std::mem::MaybeUninit::uninit();
        // dladdr's path is NUL-terminated and valid while this module is loaded.
        let path = unsafe {
            if libc::dladdr(compiler_key as *const () as *const _, info.as_mut_ptr()) == 0 {
                return None;
            }
            let name = info.assume_init().dli_fname;
            if name.is_null() {
                return None;
            }
            Path::new(std::ffi::OsStr::from_bytes(CStr::from_ptr(name).to_bytes()))
        };
        let mut hash = Sha256::new();
        std::io::copy(&mut fs::File::open(path).ok()?, &mut hash).ok()?;
        Some(hash.finalize().into())
    })
}

#[cfg(not(unix))]
fn compiler_key() -> Option<Key> {
    None
}

fn key(input: &impl Serialize) -> Option<Key> {
    let mut hash = Sha256::new();
    serde_json::to_writer(&mut hash, input).ok()?;
    Some(hash.finalize().into())
}

pub(super) struct Entry {
    path: PathBuf,
    source: Key,
}

impl Entry {
    pub(super) fn new(dir: &Path, path: &str, owner: &TypeId, pax: &str) -> Option<Self> {
        let slot = key(&(path, owner))?;
        let path = dir.join(format!("{:x}", sha2::digest::Output::<Sha256>::from(slot)));
        let source = Sha256::new()
            .chain_update(compiler_key()?)
            .chain_update(slot)
            .chain_update(pax.as_bytes())
            .finalize()
            .into();
        Some(Self { path, source })
    }

    pub(super) fn tags<E>(
        &self,
        parse: impl FnOnce() -> Result<Vec<String>, E>,
    ) -> Result<Vec<String>, E> {
        let path = self.path.with_extension("tags");
        if let Some(tags) = Self::read(&path, &self.source) {
            return Ok(tags);
        }
        let tags = parse()?;
        Self::write(&path, &self.source, &tags);
        Ok(tags)
    }

    pub(super) fn bindings(&self, context: &TemplateNodeParseContext) -> Option<Key> {
        // Fingerprint the actual lowering inputs, independent of map order.
        // Use pairs instead of JSON objects since TypeId isn't a string key.
        fn sorted<K: Ord, V>(map: &std::collections::HashMap<K, V>) -> Vec<(&K, &V)> {
            let mut pairs: Vec<_> = map.iter().collect();
            pairs.sort_unstable_by_key(|(key, _)| *key);
            pairs
        }
        key(&(
            self.source,
            sorted(&context.pascal_identifier_to_type_id_map),
            sorted(&context.route_branch_descriptors),
            sorted(&context.content_properties),
        ))
    }

    pub(super) fn lower(&self, pax: &str, context: TemplateNodeParseContext) -> LoweredTemplate {
        let Some(key) = self.bindings(&context) else {
            return LoweredTemplate::parse(pax, context);
        };
        let path = self.path.with_extension("template");
        if let Some(value) = Self::read(&path, &key) {
            return value;
        }
        let value = LoweredTemplate::parse(pax, context);
        Self::write(&path, &key, &value);
        value
    }

    fn read<T: DeserializeOwned>(path: &Path, key: &Key) -> Option<T> {
        let bytes = fs::read(path).ok()?;
        let data = bytes.get(64..)?;
        if bytes[..32] != key[..] || bytes[32..64] != Sha256::digest(data)[..] {
            return None;
        }
        serde_json::from_slice(data).ok()
    }

    fn write(path: &Path, key: &Key, value: &impl Serialize) -> Option<()> {
        // Keep one last-good entry per component/lane, atomically replaced so
        // concurrent builds can lose a hit but never consume a partial result.
        let data = serde_json::to_vec(value).ok()?;
        let dir = path.parent()?;
        fs::create_dir_all(dir).ok()?;
        let mut file = tempfile::NamedTempFile::new_in(dir).ok()?;
        file.write_all(key).ok()?;
        file.write_all(&Sha256::digest(&data)).ok()?;
        file.write_all(&data).ok()?;
        file.persist(path).ok()?;
        Some(())
    }
}

#[cfg(test)]
mod tests;
