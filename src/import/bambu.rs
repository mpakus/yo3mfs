//! Bambu metadata is optional. Geometry stays in Core/Production model parts.
use super::{ImportError, invalid, xml::Tag};
use crate::i18n::Key;
use std::collections::HashMap;

#[derive(Default)]
pub struct Metadata {
    pub roles: HashMap<u32, String>,
    pub parts: HashMap<(u32, u32), String>,
    pub plates: HashMap<(u32, u32), u32>,
}
#[derive(Default)]
pub struct Parser {
    data: Metadata,
    object: Option<u32>,
    part: Option<u32>,
    plate: Option<u32>,
    instance: HashMap<String, String>,
}
impl Parser {
    pub fn event(
        &mut self,
        t: &Tag,
        path: &[(String, String)],
        end: bool,
    ) -> Result<(), ImportError> {
        let parent = path
            .iter()
            .rev()
            .nth(1)
            .map(|(_, n)| n.as_str())
            .unwrap_or("");
        if !end {
            match t.name.as_str() {
                "object" if parent == "config" => self.object = Some(t.id("id")?),
                "part" => {
                    self.part = Some(t.id("id")?);
                    if let (Some(o), Some(p), Some(kind)) =
                        (self.object, self.part, t.attr("subtype"))
                    {
                        self.data.parts.insert((o, p), kind.into());
                    }
                }
                "plate" => self.plate = None,
                "model_instance" => self.instance.clear(),
                "metadata" => {
                    let (Some(key), Some(value)) = (t.attr("key"), t.attr("value")) else {
                        return Ok(());
                    };
                    match parent {
                        "plate" if key == "plater_id" => self.plate = value.parse().ok(),
                        "model_instance" => {
                            self.instance.insert(key.into(), value.into());
                        }
                        "part" if matches!(key, "type" | "subtype") => {
                            if let (Some(o), Some(p)) = (self.object, self.part) {
                                self.data.parts.insert((o, p), value.into());
                            }
                        }
                        "object" if key == "type" => {
                            if let Some(o) = self.object {
                                self.data.roles.insert(o, value.into());
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } else {
            match t.name.as_str() {
                "object" => self.object = None,
                "part" => self.part = None,
                "model_instance" => {
                    if let (Some(plate), Some(object), Some(instance)) = (
                        self.plate,
                        self.instance.get("object_id").and_then(|v| v.parse().ok()),
                        self.instance
                            .get("instance_id")
                            .and_then(|v| v.parse().ok()),
                    ) && let Some(old) = self.data.plates.insert((object, instance), plate)
                        && old != plate
                    {
                        return Err(invalid(Key::AmbiguousPlate));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn finish(self) -> Metadata {
        self.data
    }
}
