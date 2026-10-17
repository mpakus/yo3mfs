use super::{ImportError, Limits, MODEL_REL, bambu, check_cancel, invalid, model, xml};
use crate::i18n::{Key, Message};
use std::{
    collections::{HashMap, HashSet},
    io::{BufReader, Read, Seek, SeekFrom},
    sync::atomic::AtomicBool,
};

pub struct Package<'a, R> {
    zip: zip::ZipArchive<R>,
    names: HashMap<String, usize>,
    limits: &'a Limits,
    cancel: &'a AtomicBool,
    expanded: u64,
    pub index_bytes: usize,
}

/// Resolve a package URI, never an operating-system path.
pub fn resolve(base: &str, target: &str) -> Result<String, ImportError> {
    if target.contains(['\\', ':', '\0', '?', '#']) {
        return Err(invalid(Key::BadPackagePath));
    }
    let mut decoded = Vec::new();
    let bytes = target.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = target
                .get(i + 1..i + 3)
                .ok_or_else(|| invalid(Key::BadUri))?;
            decoded.push(u8::from_str_radix(hex, 16).map_err(|_| invalid(Key::BadUri))?);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    let decoded = String::from_utf8(decoded).map_err(|_| invalid(Key::BadUriUtf8))?;
    if decoded.contains(['\\', ':', '\0', '?', '#']) {
        return Err(invalid(Key::ForbiddenUri));
    }
    let joined = if decoded.starts_with('/') {
        decoded.trim_start_matches('/').to_string()
    } else {
        format!(
            "{}{}",
            base.rsplit_once('/')
                .map(|(p, _)| format!("{p}/"))
                .unwrap_or_default(),
            decoded
        )
    };
    let mut parts = Vec::new();
    for p in joined.split('/') {
        match p {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return Err(invalid(Key::PathEscape));
                }
            }
            _ => parts.push(p),
        }
    }
    if parts.is_empty() {
        return Err(invalid(Key::EmptyPath));
    }
    Ok(parts.join("/"))
}
impl<'a, R: Read + Seek> Package<'a, R> {
    pub fn new(
        mut input: R,
        limits: &'a Limits,
        cancel: &'a AtomicBool,
    ) -> Result<Self, ImportError> {
        if input.seek(SeekFrom::End(0))? > limits.archive_bytes {
            return Err(ImportError::Limit(Key::ArchiveSize));
        }
        input.seek(SeekFrom::Start(0))?;
        let index_bytes = super::zip_index::preflight(&mut input, limits)?;
        let mut zip = zip::ZipArchive::new(input)?;
        if zip.len() > limits.entries {
            return Err(ImportError::Limit(Key::ZipEntries));
        }
        let mut names = HashMap::new();
        for i in 0..zip.len() {
            check_cancel(cancel)?;
            let f = zip.by_index(i)?;
            if f.is_dir() {
                continue;
            }
            let name = resolve("", &f.name()?)?;
            if names.insert(name, i).is_some() {
                return Err(invalid(Key::DuplicateZipEntry));
            }
        }
        Ok(Self {
            zip,
            names,
            limits,
            cancel,
            expanded: 0,
            index_bytes,
        })
    }
    fn xml(
        &mut self,
        name: &str,
        visit: impl FnMut(&xml::Tag, &[(String, String)], bool) -> Result<(), ImportError>,
    ) -> Result<(), ImportError> {
        check_cancel(self.cancel)?;
        let index = *self
            .names
            .get(name)
            .ok_or_else(|| invalid(Message::with(Key::MissingPart, [name])))?;
        let f = self.zip.by_index(index)?;
        if f.size() > self.limits.xml_bytes {
            return Err(ImportError::Limit(Key::XmlSize));
        }
        let allowed = self
            .limits
            .xml_bytes
            .min(self.limits.expanded_bytes.saturating_sub(self.expanded));
        let mut reader = Bounded {
            inner: f,
            read: 0,
            max: allowed,
            cancel: self.cancel,
            exceeded: false,
        };
        let result = xml::walk(BufReader::new(&mut reader), self.limits, visit);
        self.expanded += reader.read;
        if reader.exceeded {
            return Err(ImportError::Limit(Key::ExpandedData));
        }
        check_cancel(self.cancel)?;
        result
    }
    pub fn relationships(&mut self, part: &str) -> Result<Vec<String>, ImportError> {
        let rel = if part.is_empty() {
            "_rels/.rels".to_string()
        } else {
            let (dir, file) = part.rsplit_once('/').unwrap_or(("", part));
            format!("{dir}/_rels/{file}.rels")
                .trim_start_matches('/')
                .to_string()
        };
        if !self.names.contains_key(&rel) {
            if part.is_empty() {
                return Err(invalid(Key::MissingRels));
            }
            return Ok(vec![]);
        }
        let mut result = Vec::new();
        let mut ids = HashSet::new();
        self.xml(&rel, |tag, path, end| {
            if end {
                return Ok(());
            }
            if path.len() == 1
                && (tag.name != "Relationships"
                    || tag.ns != "http://schemas.openxmlformats.org/package/2006/relationships")
            {
                return Err(invalid(Key::BadRelsRoot));
            }
            if tag.name == "Relationship" && path.len() == 2 {
                if tag.attr("TargetMode") == Some("External") {
                    return Err(invalid(Key::ExternalRels));
                }
                if !ids.insert(tag.need("Id")?.to_string()) {
                    return Err(invalid(Key::DuplicateRel));
                }
                let target = resolve(part, tag.need("Target")?)?;
                if tag.attr("Type") == Some(MODEL_REL) {
                    result.push(target);
                }
            }
            Ok(())
        })?;
        Ok(result)
    }
    pub fn model(
        &mut self,
        part: &str,
        budget: &mut model::Budget,
        warnings: &mut Vec<Message>,
    ) -> Result<model::Model, ImportError> {
        let mut parser =
            model::Parser::new(part, self.cancel, self.limits.clone(), budget, warnings);
        self.xml(part, |tag, path, end| parser.event(tag, path, end))?;
        parser.finish()
    }
    pub fn bambu(
        &mut self,
        warnings: &mut Vec<Message>,
        budget: &mut model::Budget,
    ) -> Result<bambu::Metadata, ImportError> {
        let mut parser = bambu::Parser::default();
        let limits = self.limits.clone();
        if self.names.contains_key("Metadata/model_settings.config") {
            self.xml("Metadata/model_settings.config", |tag, path, end| {
                if !end {
                    budget.charge(
                        256 + tag.attrs.values().map(|v| v.len() * 2).sum::<usize>(),
                        &limits,
                    )?;
                }
                parser.event(tag, path, end)
            })?;
        }
        if self.names.contains_key("Metadata/project_settings.config") {
            warnings.push(Key::BambuWarning.into());
        }
        Ok(parser.finish())
    }
}
struct Bounded<'a, R> {
    inner: R,
    read: u64,
    max: u64,
    cancel: &'a AtomicBool,
    exceeded: bool,
}
impl<R: Read> Read for Bounded<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(std::io::Error::other("Cancelled"));
        }
        let size = buf
            .len()
            .min(32768)
            .min(self.max.saturating_sub(self.read).saturating_add(1) as usize);
        let n = self.inner.read(&mut buf[..size])?;
        self.read += n as u64;
        if self.read > self.max {
            self.exceeded = true;
            return Err(std::io::Error::other("Expanded data limit exceeded"));
        }
        Ok(n)
    }
}
