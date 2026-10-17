use super::{ImportError, Limits, invalid};
use crate::i18n::{Key, Message};
use quick_xml::{
    NsReader,
    events::Event,
    name::{QName, ResolveResult},
};
use std::{
    collections::HashMap,
    io::{BufRead, Read},
};

#[derive(Default)]
pub struct Tag {
    pub name: String,
    pub ns: String,
    pub attrs: HashMap<(String, String), String>,
    pub required: Vec<String>,
}
impl Tag {
    pub fn attr(&self, key: &str) -> Option<&str> {
        self.attr_ns("", key)
    }
    pub fn attr_ns(&self, ns: &str, key: &str) -> Option<&str> {
        self.attrs.get(&(ns.into(), key.into())).map(String::as_str)
    }
    pub fn need(&self, key: &str) -> Result<&str, ImportError> {
        self.attr(key)
            .ok_or_else(|| invalid(Message::with(Key::MissingAttribute, [key, &self.name])))
    }
    pub fn id(&self, key: &str) -> Result<u32, ImportError> {
        let id = self
            .need(key)?
            .parse::<u32>()
            .map_err(|_| invalid(Message::with(Key::InvalidAttribute, [key])))?;
        if id == 0 {
            return Err(invalid(Key::PositiveId));
        }
        Ok(id)
    }
}
fn namespace(result: ResolveResult<'_>) -> Result<String, ImportError> {
    match result {
        ResolveResult::Bound(ns) => Ok(ns.as_ref().to_string()),
        ResolveResult::Unbound => Ok(String::new()),
        ResolveResult::Unknown(p) => Err(invalid(Message::with(Key::UnknownNamespace, [p]))),
    }
}
pub fn walk<R: BufRead>(
    input: R,
    limits: &Limits,
    mut visit: impl FnMut(&Tag, &[(String, String)], bool) -> Result<(), ImportError>,
) -> Result<(), ImportError> {
    // Limit allocation *before* quick-xml buffers a whole comment/tag/text token.
    let input = EventBound {
        inner: input,
        used: 0,
        max: (limits.memory_bytes / 8).clamp(128, 1024 * 1024),
    };
    let mut reader = NsReader::from_reader(input);
    reader.resolver_mut().set_max_namespace_bindings(64);
    reader.config_mut().check_end_names = true;
    let mut buf = Vec::new();
    let mut path = Vec::new();
    let mut roots = 0;
    loop {
        reader.get_mut().used = 0;
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| invalid(Message::with(Key::RawDetail, [e])))?;
        match event {
            Event::Start(ref e) | Event::Empty(ref e) => {
                if path.len() >= limits.xml_depth {
                    return Err(ImportError::Limit(Key::XmlDepth));
                }
                if path.is_empty() {
                    roots += 1;
                    if roots != 1 {
                        return Err(invalid(Key::XmlRoots));
                    }
                }
                let (ns, local) = reader.resolver().resolve_element(e.name());
                let mut tag = Tag {
                    name: local.as_ref().to_string(),
                    ns: namespace(ns)?,
                    ..Tag::default()
                };
                for a in e.attributes() {
                    let a = a.map_err(|e| invalid(Message::with(Key::RawDetail, [e])))?;
                    if a.key.as_ref().len() > 256 {
                        return Err(ImportError::Limit(Key::XmlNameLength));
                    }
                    if a.key.as_ref() == "xmlns" || a.key.as_ref().starts_with("xmlns:") {
                        if a.value.len() > 2048 {
                            return Err(ImportError::Limit(Key::NamespaceLength));
                        }
                        continue;
                    }
                    let (ns, key) = reader.resolver().resolve_attribute(a.key);
                    let value = a
                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|e| invalid(Message::with(Key::RawDetail, [e])))?
                        .into_owned();
                    if value.len() > 65536 || tag.attrs.len() >= 64 {
                        return Err(ImportError::Limit(Key::XmlAttributes));
                    }
                    let key = (namespace(ns)?, key.as_ref().to_string());
                    if tag.attrs.insert(key, value).is_some() {
                        return Err(invalid(Key::DuplicateAttribute));
                    }
                }
                if let Some(required) = tag.attr("requiredextensions").map(str::to_string) {
                    for p in required.split_whitespace() {
                        let ns = namespace(
                            reader
                                .resolver()
                                .resolve_element(QName(&format!("{p}:extension")))
                                .0,
                        )?;
                        if ns.is_empty() {
                            return Err(invalid(Key::UndeclaredExtension));
                        }
                        tag.required.push(ns);
                    }
                }
                path.push((tag.ns.clone(), tag.name.clone()));
                visit(&tag, &path, false)?;
                if matches!(event, Event::Empty(_)) {
                    visit(&tag, &path, true)?;
                    path.pop();
                }
            }
            Event::End(e) => {
                let (ns, local) = reader.resolver().resolve_element(e.name());
                let tag = Tag {
                    name: local.as_ref().to_string(),
                    ns: namespace(ns)?,
                    ..Tag::default()
                };
                if path.is_empty() {
                    return Err(invalid(Key::XmlEnd));
                }
                visit(&tag, &path, true)?;
                path.pop();
            }
            Event::DocType(_) => return Err(invalid(Key::ForbiddenDtd)),
            Event::GeneralRef(e) => {
                let name = e.as_ref();
                if !matches!(name, "amp" | "lt" | "gt" | "apos" | "quot") && !name.starts_with('#')
                {
                    return Err(invalid(Key::ForbiddenEntity));
                }
            }
            Event::Text(e) if path.is_empty() && !e.as_ref().chars().all(char::is_whitespace) => {
                return Err(invalid(Key::XmlOutsideText));
            }
            Event::Eof => {
                if !path.is_empty() || roots != 1 {
                    return Err(invalid(Key::XmlIncomplete));
                }
                break;
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(())
}

struct EventBound<R> {
    inner: R,
    used: usize,
    max: usize,
}
impl<R: BufRead> Read for EventBound<R> {
    fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
        let available = self.fill_buf()?;
        let n = out.len().min(available.len());
        out[..n].copy_from_slice(&available[..n]);
        self.consume(n);
        Ok(n)
    }
}
impl<R: BufRead> BufRead for EventBound<R> {
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.used >= self.max {
            return Err(std::io::Error::other("XML token size limit exceeded"));
        }
        let bytes = self.inner.fill_buf()?;
        Ok(&bytes[..bytes.len().min(self.max - self.used)])
    }
    fn consume(&mut self, n: usize) {
        self.used += n;
        self.inner.consume(n);
    }
}
