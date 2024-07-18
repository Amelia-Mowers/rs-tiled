use std::{collections::HashMap, str::FromStr};

use quick_xml::events::{attributes::Attribute, Event};

use crate::{
    error::{Error, Result},
    parse::xml::{Parser, Reader},
    util::{get_attrs, parse_tag},
};

/// Represents a RGBA color with 8-bit depth on each channel.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[allow(missing_docs)]
pub struct Color {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl FromStr for Color {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Color, Self::Err> {
        let s = if let Some(stripped) = s.strip_prefix('#') {
            stripped
        } else {
            s
        };
        match s.len() {
            6 if s.is_ascii() => {
                let r = u8::from_str_radix(&s[0..2], 16);
                let g = u8::from_str_radix(&s[2..4], 16);
                let b = u8::from_str_radix(&s[4..6], 16);
                match (r, g, b) {
                    (Ok(red), Ok(green), Ok(blue)) => Ok(Color {
                        alpha: 0xFF,
                        red,
                        green,
                        blue,
                    }),
                    _ => Err(()),
                }
            }
            8 if s.is_ascii() => {
                let a = u8::from_str_radix(&s[0..2], 16);
                let r = u8::from_str_radix(&s[2..4], 16);
                let g = u8::from_str_radix(&s[4..6], 16);
                let b = u8::from_str_radix(&s[6..8], 16);
                match (a, r, g, b) {
                    (Ok(alpha), Ok(red), Ok(green), Ok(blue)) => Ok(Color {
                        alpha,
                        red,
                        green,
                        blue,
                    }),
                    _ => Err(()),
                }
            }
            _ => Err(()),
        }
    }
}

/// Represents a custom property's value.
///
/// Also read the [TMX docs](https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tmx-properties).
#[derive(Debug, PartialEq, Clone)]
pub enum PropertyValue {
    /// A boolean value. Corresponds to the `bool` property type.
    BoolValue(bool),
    /// A floating point value. Corresponds to the `float` property type.
    FloatValue(f32),
    /// A signed integer value. Corresponds to the `int` property type.
    IntValue(i32),
    /// A color value. Corresponds to the `color` property type.
    ColorValue(Color),
    /// A string value. Corresponds to the `string` property type.
    StringValue(String),
    /// A filepath value. Corresponds to the `file` property type.
    /// Holds the path relative to the map or tileset.
    FileValue(String),
    /// An object ID value. Corresponds to the `object` property type.
    /// Holds the id of a referenced object, or 0 if unset.
    ObjectValue(u32),
    /// A class value. Corresponds to the `class` property type.
    /// Holds the type name and a set of properties.
    ClassValue {
        /// The type name.
        property_type: String,
        /// A set of properties.
        properties: Properties,
    },
}

impl PropertyValue {
    fn new(property_type: String, value: String) -> Result<PropertyValue> {
        // Check the property type against the value.
        match property_type.as_str() {
            "bool" => match value.parse() {
                Ok(val) => Ok(PropertyValue::BoolValue(val)),
                Err(err) => Err(Error::InvalidPropertyValue {
                    description: err.to_string(),
                }),
            },
            "float" => match value.parse() {
                Ok(val) => Ok(PropertyValue::FloatValue(val)),
                Err(err) => Err(Error::InvalidPropertyValue {
                    description: err.to_string(),
                }),
            },
            "int" => match value.parse() {
                Ok(val) => Ok(PropertyValue::IntValue(val)),
                Err(err) => Err(Error::InvalidPropertyValue {
                    description: err.to_string(),
                }),
            },
            "color" if value.len() > 1 => Color::from_str(&value)
                .map(PropertyValue::ColorValue)
                .map_err(|_| Error::InvalidPropertyValue {
                    description: "Couldn't parse color".to_string(),
                }),
            "string" => Ok(PropertyValue::StringValue(value)),
            "object" => match value.parse() {
                Ok(val) => Ok(PropertyValue::ObjectValue(val)),
                Err(err) => Err(Error::InvalidPropertyValue {
                    description: err.to_string(),
                }),
            },
            "file" => Ok(PropertyValue::FileValue(value)),
            _ => Err(Error::UnknownPropertyType {
                type_name: property_type,
            }),
        }
    }
}

/// A custom property container.
pub type Properties = HashMap<String, PropertyValue>;

pub(crate) async fn parse_properties<R: Reader>(parser: &mut Parser<R>) -> Result<Properties> {
    let mut p = HashMap::new();
    let mut buffer = Vec::new();
    parse_tag!(parser => &mut buffer, "properties", {
        "property" => |attrs| {
            // add indirection because the returned async state machine is a recursive data structure
            // (`parse_properties_inner` calls `parse_properties` again)
            Box::pin(parse_properties_inner(parser, &mut p, attrs)).await
        },
    });
    Ok(p)
}

async fn parse_properties_inner<R: Reader>(
    parser: &mut Parser<R>,
    p: &mut HashMap<String, PropertyValue>,
    attrs: Vec<Attribute<'_>>,
) -> Result<()> {
    let (t, v_attr, k, p_t) = get_attrs!(
        for attr in attrs {
            Some("type") => obj_type = attr,
            Some("value") => value = attr,
            Some("propertytype") => propertytype = attr,
            "name" => name = attr
        }
        (obj_type, value, name, propertytype)
    );
    let t = t.unwrap_or("string").to_string();
    if t == "class" {
        // Class properties will have their member values stored in a nested <properties>
        // element. Only the actually set members are saved. When no members have been set
        // the properties element is left out entirely.
        let properties = if has_properties_tag_next(parser).await {
            parse_properties(parser).await?
        } else {
            HashMap::new()
        };
        p.insert(
            k.to_string(),
            PropertyValue::ClassValue {
                property_type: p_t.unwrap_or_default().to_string(),
                properties,
            },
        );
        return Ok(());
    }

    let v: String = match v_attr {
        Some(val) => val.to_string(),
        None => {
            // if the "value" attribute was missing, might be a multiline string
            match parser.read_event().await {
                Ok(Event::Text(text)) => {
                    let text = text.into_inner();
                    let text = std::str::from_utf8(&text)
                        .map_err(|err| Error::XmlDecodingError(err.into()))?;
                    Ok(text.to_string())
                }
                Err(err) => Err(Error::XmlDecodingError(err)),
                _ => Err(Error::MalformedAttributes(format!(
                    "property '{}' is missing a value",
                    k
                ))),
            }?
        }
    };

    p.insert(k.to_string(), PropertyValue::new(t, v)?);
    Ok(())
}

/// Checks if there is a properties tag next in the parser. Will consume any whitespace or comments.
async fn has_properties_tag_next<R: Reader>(parser: &mut Parser<R>) -> bool {
    // TODO: tests

    loop {
        let Ok(next) = parser.read_event().await else {
            break;
        };
        match next {
            Event::Start(start) if start.local_name().into_inner() == b"properties" => return true,

            // Ignore whitespace
            Event::Text(mut text) => {
                text.inplace_trim_start();
                text.inplace_trim_end();
                if text.is_empty() {
                    continue;
                } else {
                    // If we encounter anything else than whitespace, comments or the properties tag, we
                    // assume there are no properties.
                    return false;
                }
            }

            // Ignore comments
            Event::Comment(_) => continue,

            // If we encounter anything else than whitespace, comments or the properties tag, we
            // assume there are no properties.
            _ => return false,
        }
    }
    false
}
