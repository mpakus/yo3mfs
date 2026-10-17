//! Bambu metadata is optional. Geometry stays in Core/Production model parts.
use super::{ImportError, invalid, xml::Tag};
use crate::i18n::Key;
use serde::{
    Deserialize, Deserializer,
    de::{SeqAccess, Visitor},
};
use std::collections::HashMap;

#[derive(Default)]
pub struct Metadata {
    pub roles: HashMap<u32, String>,
    pub parts: HashMap<(u32, u32), String>,
    pub plates: HashMap<(u32, u32), u32>,
    pub object_filaments: HashMap<u32, u16>,
    pub part_filaments: HashMap<(u32, u32), u16>,
    pub palette: Vec<Option<[u8; 3]>>,
    pub color_warning: bool,
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
                        "object" | "part" if key == "extruder" => {
                            match value.parse::<u16>() {
                                Ok(0) => {} // Inherit the parent object's filament.
                                Ok(f @ 1..=256) => {
                                    if let Some(o) = self.object {
                                        if parent == "part" {
                                            if let Some(p) = self.part {
                                                self.data.part_filaments.insert((o, p), f);
                                            }
                                        } else {
                                            self.data.object_filaments.insert(o, f);
                                        }
                                    }
                                }
                                _ => self.data.color_warning = true,
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

#[derive(Default, Deserialize)]
struct ProjectColors {
    #[serde(default, deserialize_with = "bounded_strings")]
    filament_colour: Vec<String>,
    #[serde(default, deserialize_with = "bounded_strings")]
    filament_multi_colour: Vec<String>,
    #[serde(default, deserialize_with = "bounded_strings")]
    filament_colour_type: Vec<String>,
}

fn bounded_strings<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    struct Strings;
    impl<'de> Visitor<'de> for Strings {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("an array with at most 256 filament values")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut result = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                if result.len() == 256 {
                    return Err(serde::de::Error::custom("Too many filament values"));
                }
                result.push(value);
            }
            Ok(result)
        }
    }
    deserializer.deserialize_seq(Strings)
}

pub fn project_colors(bytes: &[u8], metadata: &mut Metadata) -> Result<bool, serde_json::Error> {
    let project: ProjectColors = serde_json::from_slice(bytes)?;
    metadata.palette = project
        .filament_colour
        .iter()
        .map(|value| {
            let rgb = if value.len() == 9 && value.is_ascii() {
                // Filament appearance is opaque; retain RGB and report other alpha.
                metadata.color_warning |= !value[7..].eq_ignore_ascii_case("ff");
                crate::settings::parse_hex(&value[..7])
            } else {
                crate::settings::parse_hex(value)
            };
            metadata.color_warning |= rgb.is_none();
            rgb
        })
        .collect();
    Ok(project.filament_multi_colour.iter().any(|s| !s.is_empty())
        || project.filament_colour_type.iter().any(|s| s != "1"))
}
