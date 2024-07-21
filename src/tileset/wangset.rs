use std::collections::HashMap;

use crate::{
    error::Error,
    parse::xml::{Parser, Reader},
    properties::{parse_properties, Properties},
    util::{get_attrs, parse_tag},
    Result, TileId,
};

mod wang_color;
use quick_xml::events::attributes::Attribute;
pub use wang_color::*;
mod wang_tile;
pub use wang_tile::*;

/// Wang set's terrain brush connection type.
#[derive(Debug, PartialEq, Clone, Copy, Default)]
#[allow(missing_docs)]
pub enum WangSetType {
    Corner,
    Edge,
    #[default]
    Mixed,
}

/// Raw data belonging to a WangSet.
#[derive(Debug, PartialEq, Clone)]
pub struct WangSet {
    /// The name of the Wang set.
    pub name: String,
    /// Type of Wang set.
    pub wang_set_type: WangSetType,
    /// The tile ID of the tile representing this Wang set.
    pub tile: Option<TileId>,
    /// The colors color that can be used to define the corner and/or edge of each Wang tile.
    pub wang_colors: Vec<WangColor>,
    ///  All the Wang tiles present in this Wang set, indexed by their local IDs.
    pub wang_tiles: HashMap<TileId, WangTile>,
    /// The custom properties of this Wang set.
    pub properties: Properties,
}

impl WangSet {
    /// Reads data from XML parser to create a WangSet.
    // FIXME: was public before
    pub(crate) async fn new<R: Reader>(
        parser: &mut Parser<R>,
        attrs: Vec<Attribute<'_>>,
    ) -> Result<WangSet> {
        // Get common data
        let (name, wang_set_type, tile) = get_attrs!(
            for v in attrs {
                "name" => name ?= v.parse::<String>(),
                "type" => wang_set_type ?= v.parse::<String>(),
                "tile" => tile ?= v.parse::<i64>(),
            }
            (name, wang_set_type, tile)
        );

        let wang_set_type = match wang_set_type.as_str() {
            "corner" => WangSetType::Corner,
            "edge" => WangSetType::Edge,
            _ => WangSetType::default(),
        };
        let tile = if tile >= 0 { Some(tile as u32) } else { None };

        // Gather variable data
        let mut wang_colors = Vec::new();
        let mut wang_tiles = HashMap::new();
        let mut properties = HashMap::new();

        let mut buffer = Vec::new();
        parse_tag!(parser => &mut buffer, "wangset", {
            "wangcolor" => for attrs {
                let color = WangColor::new(parser, attrs).await?;
                wang_colors.push(color);
                Ok(())
            },
            "wangtile" => for attrs {
                let (id, t) = WangTile::new(attrs)?;
                wang_tiles.insert(id, t);
                Ok(())
            },
            "properties" => for attrs {
                properties = parse_properties(parser).await?;
                Ok(())
            },
        });

        Ok(WangSet {
            name,
            wang_set_type,
            tile,
            wang_colors,
            wang_tiles,
            properties,
        })
    }
}
