use std::{collections::HashMap, path::Path};

use crate::{
    parse::xml::{Parser, Reader},
    parse_properties,
    util::{map_wrapper, parse_tag},
    Error, Image, Properties, Result,
};

/// The raw data of an [`ImageLayer`]. Does not include a reference to its parent [`Map`](crate::Map).
#[derive(Debug, PartialEq, Clone)]
pub struct ImageLayerData {
    /// The single image this layer contains, if it exists.
    pub image: Option<Image>,
}

impl ImageLayerData {
    pub(crate) async fn new<R: Reader>(
        parser: &mut Parser<R>,
        map_path: &Path,
    ) -> Result<(Self, Properties)> {
        let mut image: Option<Image> = None;
        let mut properties = HashMap::new();

        let path_relative_to = map_path.parent().ok_or(Error::PathIsNotFile)?;

        let mut buffer = Vec::new();
        parse_tag!(parser => &mut buffer, "imagelayer", {
            "image" => for attrs {
                image = Some(Image::new(parser, attrs, path_relative_to).await?);
                Ok(())
            },
            "properties" => {
                properties = parse_properties(parser).await?;
                Ok(())
            },
        });
        Ok((ImageLayerData { image }, properties))
    }
}

map_wrapper!(
    #[doc = "A layer consisting of a single image."]
    #[doc = "\nAlso see the [TMX docs](https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#imagelayer)."]
    ImageLayer => ImageLayerData
);
