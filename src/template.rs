use std::path::Path;
use std::sync::Arc;

use quick_xml::events::Event;

use crate::parse::xml::{Parser, ReadFrom, Reader};
use crate::{
    util::*, EmbeddedParseResultType, Error, MapTilesetGid, ObjectData, ResourceCache, Result,
    Tileset,
};

/// A template, consisting of an object and a tileset
///
/// Templates define a tileset and object data to use for an object that can be shared between multiple objects and
/// maps.
#[derive(Clone, Debug)]
pub struct Template {
    /// The tileset this template contains a reference to
    pub tileset: Option<Arc<Tileset>>,
    /// The object data for this template
    pub object: ObjectData,
}

impl Template {
    pub(crate) async fn parse_template(
        path: &Path,
        read_from: &mut impl ReadFrom,
        cache: &mut impl ResourceCache,
    ) -> Result<Arc<Template>> {
        // Open the template file
        let mut file =
            read_from
                .read_from(path)
                .await
                .map_err(|err| Error::ResourceLoadingError {
                    path: path.to_owned(),
                    err: Box::new(err),
                })?;
        let mut buffer = Vec::new();
        loop {
            let next = file
                .read_event_into(&mut buffer)
                .await
                .map_err(Error::XmlDecodingError)?;
            match next {
                Event::Start(start) if start.local_name().into_inner() == b"template" => {
                    let template = Self::parse_external_template(
                        &mut Parser::with_reader(file),
                        path,
                        read_from,
                        cache,
                    )
                    .await?;
                    return Ok(template);
                }
                Event::Eof => {
                    return Err(Error::PrematureEnd(
                        "Template Document ended before template element was parsed".to_string(),
                    ))
                }
                _ => {}
            }
        }
    }

    async fn parse_external_template<R: Reader>(
        parser: &mut Parser<R>,
        template_path: &Path,
        read_from: &mut impl ReadFrom,
        cache: &mut impl ResourceCache,
    ) -> Result<Arc<Template>> {
        let mut object = Option::None;
        let mut tileset = None;
        let mut tileset_gid: Vec<MapTilesetGid> = vec![];

        let mut buffer = Vec::new();
        parse_tag!(parser => &mut buffer, "template", {
            "object" => for attrs {
                object = Some(ObjectData::new(
                    parser,
                    attrs,
                    Some(&tileset_gid),
                    tileset.clone(),
                    template_path.parent().ok_or(Error::PathIsNotFile)?,
                    read_from,
                    cache
                ).await?);
                Ok(())
            },
            "tileset" => for attrs {
                let res = Tileset::parse_xml_in_map(parser, &attrs, template_path, read_from, cache).await?;
                match res.result_type {
                    EmbeddedParseResultType::ExternalReference { tileset_path } => {
                        tileset = Some(if let Some(ts) = cache.get_tileset(&tileset_path) {
                            ts
                        } else {
                            let tileset = Arc::new(crate::parse::xml::parse_tileset(&tileset_path, read_from, cache).await?);
                            cache.insert_tileset(tileset_path.clone(), tileset.clone());
                            tileset
                        });
                    }
                    EmbeddedParseResultType::Embedded { tileset: embedded_tileset } => {
                        tileset = Some(Arc::new(embedded_tileset));
                    },
                };
                tileset_gid.push(MapTilesetGid {
                    tileset: tileset.clone().unwrap(),
                    first_gid: res.first_gid,
                });
                Ok(())
            },
        });

        let object = object.ok_or(Error::TemplateHasNoObject)?;

        Ok(Arc::new(Template { tileset, object }))
    }
}
