//! Structures related to tile animations.

use quick_xml::events::attributes::Attribute;

use crate::{
    error::{Error, Result},
    parse::xml::{Parser, Reader},
    util::{get_attrs, parse_tag},
};

/// A structure describing a [frame] of a [TMX tile animation].
///
/// [frame]: https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tmx-frame
/// [TMX tile animation]: https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#animation
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Frame {
    /// The local ID of a tile within the parent tileset.
    pub tile_id: u32,
    /// How long (in milliseconds) this frame should be displayed before advancing to the next frame.
    pub duration: u32,
}

impl Frame {
    pub(crate) fn new(attrs: Vec<Attribute>) -> Result<Frame> {
        let (tile_id, duration) = get_attrs!(
            for v in attrs {
                "tileid" => tile_id ?= v.parse::<u32>(),
                "duration" => duration ?= v.parse::<u32>(),
            }
            (tile_id, duration)
        );
        Ok(Frame { tile_id, duration })
    }
}

pub(crate) async fn parse_animation<R: Reader>(parser: &mut Parser<R>) -> Result<Vec<Frame>> {
    let mut animation = Vec::new();
    parse_tag!(parser, "animation", {
        "frame" => |attrs| {
            animation.push(Frame::new(attrs)?);
            Ok(())
        },
    });
    Ok(animation)
}
