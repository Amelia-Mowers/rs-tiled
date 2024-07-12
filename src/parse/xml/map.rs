use std::path::Path;

use itertools::Itertools;
use quick_xml::events::Event;

use super::{Parser, ReadFrom, Reader};
use crate::{Error, Map, ResourceCache, Result};

pub async fn parse_map(
    path: &Path,
    read_from: &mut impl ReadFrom,
    cache: &mut impl ResourceCache,
) -> Result<Map> {
    let mut reader =
        read_from
            .read_from(path)
            .await
            .map_err(|err| Error::ResourceLoadingError {
                path: path.to_owned(),
                err: Box::new(err),
            })?;
    let mut buffer = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buffer)
            .await
            .map_err(Error::XmlDecodingError)?
        {
            Event::Start(start) if start.local_name().into_inner() == b"map" => {
                let attributes = start
                    .attributes()
                    .try_collect()
                    .map_err(|err| Error::XmlDecodingError(err.into()))?;
                let mut parser = Parser::with_reader(reader);
                return Map::parse_xml(&mut parser, attributes, path, read_from, cache).await;
            }
            Event::Eof => {
                return Err(Error::PrematureEnd(
                    "Document ended before map was parsed".to_string(),
                ))
            }
            _ => {}
        }
    }
}
