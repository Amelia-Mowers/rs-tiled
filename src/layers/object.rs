use std::{collections::HashMap, path::Path, sync::Arc};

use quick_xml::events::attributes::Attribute;

use crate::{
    parse::xml::{Parser, ReadFrom, Reader},
    parse_properties,
    util::{get_attrs, map_wrapper, parse_tag},
    Color, Error, MapTilesetGid, Object, ObjectData, Properties, ResourceCache, Result, Tileset,
};

/// Raw data referring to a map object layer or tile collision data.
#[derive(Debug, PartialEq, Clone)]
pub struct ObjectLayerData {
    objects: Vec<ObjectData>,
    /// The color used in the editor to display objects in this layer.
    pub colour: Option<Color>,
}

impl ObjectLayerData {
    /// If it is known that there are no objects with tile images in it (i.e. collision data)
    /// then we can pass in [`None`] as the tilesets
    pub(crate) async fn new<R: Reader>(
        parser: &mut Parser<R>,
        attrs: Vec<Attribute<'_>>,
        tilesets: Option<&[MapTilesetGid]>,
        for_tileset: Option<Arc<Tileset>>,
        // path_relative_to is a directory to which all other files are relative to
        path_relative_to: &Path,
        read_from: &mut impl ReadFrom,
        cache: &mut impl ResourceCache,
    ) -> Result<(ObjectLayerData, Properties)> {
        let c = get_attrs!(
            for v in attrs {
                Some("color") => color ?= v.parse(),
            }
            color
        );
        let mut objects = Vec::new();
        let mut properties = HashMap::new();
        let mut buffer = Vec::new();
        parse_tag!(parser => &mut buffer, "objectgroup", {
            "object" => for attrs {
                objects.push(ObjectData::new(
                    parser,
                    attrs,
                    tilesets,
                    for_tileset.as_ref().cloned(),
                    path_relative_to,
                    read_from,
                    cache
                ).await?);
                Ok(())
            },
            "properties" => {
                properties = parse_properties(parser).await?;
                Ok(())
            },
        });
        Ok((ObjectLayerData { objects, colour: c }, properties))
    }

    /// Returns the data belonging to the objects contained within the layer, in the order they were
    /// declared in the TMX file.
    #[inline]
    pub fn object_data(&self) -> &[ObjectData] {
        self.objects.as_ref()
    }
}

map_wrapper!(
    #[doc = "Also called an \"object group\". Used for storing [`Object`]s in a map."]
    ObjectLayer => ObjectLayerData);

impl<'map> ObjectLayer<'map> {
    /// Obtains the object corresponding to the index given.
    pub fn get_object(&self, idx: usize) -> Option<Object<'map>> {
        self.data
            .objects
            .get(idx)
            .map(|data| Object::new(self.map, data))
    }

    /// Returns an iterator over the objects present in this layer, in the order they were declared
    /// in in the TMX file.
    ///
    /// ## Example
    /// ```
    /// # use tiled::Loader;
    /// use tiled::Object;
    ///
    /// # fn main() {
    /// # let map = Loader::new()
    /// #     .load_tmx_map("assets/tiled_group_layers.tmx")
    /// #     .unwrap();
    /// #
    /// let spawnpoints: Vec<Object> = map
    ///     .layers()
    ///     .filter_map(|layer| match layer.layer_type() {
    ///         tiled::LayerType::Objects(layer) => Some(layer),
    ///         _ => None,
    ///     })
    ///     .flat_map(|layer| layer.objects())
    ///     .filter(|object| object.user_type == "spawn")
    ///     .collect();
    ///
    /// dbg!(spawnpoints);
    /// # }
    /// ```
    #[inline]
    pub fn objects(&self) -> impl ExactSizeIterator<Item = Object<'map>> + 'map {
        let map: &'map crate::Map = self.map;
        self.data
            .objects
            .iter()
            .map(move |object| Object::new(map, object))
    }
}
