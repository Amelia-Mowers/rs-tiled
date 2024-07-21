/// Loops through the attributes once and pulls out the ones we ask it to. It
/// will check that the required ones are there.
///
/// The syntax is:
/// ```ignore
/// get_attrs!(
///     for $attr in $attributes {
///         $($branch),*
///     }
///     $expression_to_return
/// )
/// ```
/// Where `$attributes` is anything that implements `Iterator<Item = OwnedAttribute>`,
/// and `$attr` is the value of the attribute (a String) going to be used in each branch.
///
/// Each branch indicates a variable to be set once a certain attribute is found.
/// Its syntax is as follows:
/// ```ignore
/// "attribute name" => variable_name = expression_using_$attr,
/// ```
///
/// For instance:
/// ```ignore
/// "source" => source = v,
/// ```
/// The variable set has an inferred type `T`. In this case, `source` is inferred to be a `String`,
/// and `$attr` has been named `v`.
///
/// If `Some` encapsulates the attribute name (like so: `Some("attribute name")`) then the attribute
/// is meant to be optional, which will make the variable an `Option<T>` rather than `T`. Even if it
/// is technically an Option, the assignment is still done *as if it was `T`*, for instance:
/// ```ignore
/// Some("name") => name = v,
/// ```
///
/// Finally, branches can also use `?=` instead of `=`, which will make them accept a `Result<T, E>`
/// instead. If the expression results in an Err, the error will be handled internally and the
/// iteration will return early with a `Result<T, crate::Error>`.
///
/// Here are some examples of valid branches:
/// ```ignore
/// Some("spacing") => spacing ?= v.parse(),
/// Some("margin") => margin ?= v.parse(),
/// Some("columns") => columns ?= v.parse(),
/// Some("name") => name = v,
///
/// "tilecount" => tilecount ?= v.parse::<u32>(),
/// "tilewidth" => tile_width ?= v.parse::<u32>(),
/// "tileheight" => tile_height ?= v.parse::<u32>(),
/// ```
///
/// Finally, after the `for` block, `$expression_to_return` indicates what to return once the
/// iteration has finished. It may refer to variables declared previously.
///
/// ## Example
/// ```ignore
/// let ((c, infinite), (v, o, w, h, tw, th)) = get_attrs!(
///     for v in attrs {
///         Some("backgroundcolor") => colour ?= v.parse(),
///         Some("infinite") => infinite = v == "1",
///         "version" => version = v,
///         "orientation" => orientation ?= v.parse::<Orientation>(),
///         "width" => width ?= v.parse::<u32>(),
///         "height" => height ?= v.parse::<u32>(),
///         "tilewidth" => tile_width ?= v.parse::<u32>(),
///         "tileheight" => tile_height ?= v.parse::<u32>(),
///     }
///     ((colour, infinite), (version, orientation, width, height, tile_width, tile_height))
/// );
/// ```
macro_rules! get_attrs {
    (
        for $attr:ident in $attrs:ident {
            $($branches:tt)*
        }
        $ret_expr:expr
    ) => {
        {
            $crate::util::let_attr_branches!($($branches)*);

            for attr in $attrs.iter() {
                let $attr = std::str::from_utf8(&attr.value).map_err(|err| {
                    $crate::error::Error::XmlDecodingError(quick_xml::Error::NonDecodable(Some(err)))
                })?;
                $crate::util::process_attr_branches!(attr; $($branches)*);
            }

            $crate::util::handle_attr_branches!($($branches)*);

            $ret_expr
        }
    };
}

macro_rules! let_attr_branches {
    () => {};

    (Some($attr_pat_opt:literal) => $opt_var:ident $(?)?= $opt_expr:expr $(, $($tail:tt)*)?) => {
        let mut $opt_var = None;
        $crate::util::let_attr_branches!($($($tail)*)?);
    };

    ($attr_pat_opt:literal => $opt_var:ident $(?)?= $opt_expr:expr $(, $($tail:tt)*)?) => {
        let mut $opt_var = None;
        $crate::util::let_attr_branches!($($($tail)*)?);
    };
}

pub(crate) use let_attr_branches;

macro_rules! process_attr_branches {
    ($attr:ident; ) => {};

    ($attr:ident; Some($attr_pat_opt:literal) => $opt_var:ident = $opt_expr:expr $(, $($tail:tt)*)?) => {
        if($attr.key.local_name().into_inner() == $attr_pat_opt.as_bytes()) {
            $opt_var = Some($opt_expr);
        }
        else {
            $crate::util::process_attr_branches!($attr; $($($tail)*)?);
        }
    };

    ($attr:ident; Some($attr_pat_opt:literal) => $opt_var:ident ?= $opt_expr:expr $(, $($tail:tt)*)?) => {
        if($attr.key.local_name().into_inner() == $attr_pat_opt.as_bytes()) {
            $opt_var = Some($opt_expr.map_err(|_|
                $crate::Error::MalformedAttributes(
                    concat!("Error parsing optional attribute '", $attr_pat_opt, "'").to_owned()
                )
            )?);
        }
        else {
            $crate::util::process_attr_branches!($attr; $($($tail)*)?);
        }
    };

    ($attr:ident; $attr_pat_opt:literal => $opt_var:ident = $opt_expr:expr $(, $($tail:tt)*)?) => {
        if($attr.key.local_name().into_inner() == $attr_pat_opt.as_bytes()) {
            $opt_var = Some($opt_expr);
        }
        else {
            $crate::util::process_attr_branches!($attr; $($($tail)*)?);
        }
    };

    ($attr:ident; $attr_pat_opt:literal => $opt_var:ident ?= $opt_expr:expr $(, $($tail:tt)*)?) => {
        if($attr.key.local_name().into_inner() == $attr_pat_opt.as_bytes()) {
            $opt_var = Some($opt_expr.map_err(|_|
                $crate::Error::MalformedAttributes(
                    concat!("Error parsing attribute '", $attr_pat_opt, "'").to_owned()
                )
            )?);
        }
        else {
            $crate::util::process_attr_branches!($attr; $($($tail)*)?);
        }
    }
}

pub(crate) use process_attr_branches;

macro_rules! handle_attr_branches {
    () => {};

    (Some($attr_pat_opt:literal) => $opt_var:ident $(?)?= $opt_expr:expr $(, $($tail:tt)*)?) => {
        $crate::util::handle_attr_branches!($($($tail)*)?);
    };

    ($attr_pat_opt:literal => $opt_var:ident $(?)?= $opt_expr:expr $(, $($tail:tt)*)?) => {
        let $opt_var = $opt_var
            .ok_or_else(||
                Error::MalformedAttributes(
                    concat!("Missing attribute: ", $attr_pat_opt).to_owned()
                )
            )?;

        $crate::util::handle_attr_branches!($($($tail)*)?);
    };
}

pub(crate) use handle_attr_branches;

/// Goes through the children of the tag and will call the correct function for
/// that child. Closes the tag.
macro_rules! parse_tag {
    (@match_next $next:expr, $close_tag:expr, {$($open_tag:expr => $( for $attrs:ident )? $body:block),* $(,)*}) => {
        match $next {
            #[allow(unused_variables)]
            quick_xml::events::Event::Start(start) | quick_xml::events::Event::Empty(start) => {
                $(
                    if start.local_name().into_inner() == $open_tag.as_bytes() {
                        $(
                            let $attrs = start
                                .attributes()
                                .collect::<std::result::Result<std::vec::Vec<_>, _>>()
                                .map_err(|err| $crate::Error::XmlDecodingError(err.into()))?;
                        )?
                        $body?
                    }
                )*
            }

            quick_xml::events::Event::End(end) if end.local_name().into_inner() == $close_tag.as_bytes() => {
                break;
            }

            quick_xml::events::Event::Eof => {
                return Err(Error::PrematureEnd("Document ended before we expected.".to_string()));
            }

            _ => {}
        }
    };

    ($parser:expr, $close_tag:expr, {$($open_tag:expr => $( for $attrs:ident )? $body:block),* $(,)*}) => {
        if !$parser.last_event_was_empty {
            loop {
                let next: quick_xml::events::Event = $parser.read_event().await.map_err(Error::XmlDecodingError)?;
                parse_tag!(@match_next next, $close_tag, { $($open_tag => $( for $attrs )? $body, )? })
            }
        }
    };

    ($parser:expr => $buf:expr, $close_tag:expr, {$($open_tag:expr => $( for $attrs:ident )? $body:block),* $(,)*}) => {
        if !$parser.last_event_was_empty {
            loop {
                let next: quick_xml::events::Event = $parser.read_event_into($buf).await.map_err(Error::XmlDecodingError)?;
                parse_tag!(@match_next next, $close_tag, { $($open_tag => $( for $attrs )? $body, )? })
            }
        }
    }
}

/// Creates a new type that wraps an internal data type over along with a map.
macro_rules! map_wrapper {
    ($(#[$attrs:meta])* $name:ident => $data_ty:ty) => {
        #[derive(Clone, Copy, PartialEq, Debug)]
        $(#[$attrs])*
        pub struct $name<'map> {
            pub(crate) map: &'map $crate::Map,
            pub(crate) data: &'map $data_ty,
        }

        impl<'map> $name<'map> {
            #[inline]
            pub(crate) fn new(map: &'map $crate::Map, data: &'map $data_ty) -> Self {
                Self { map, data }
            }

            /// Get the map this object is from.
            #[inline]
            pub fn map(&self) -> &'map $crate::Map {
                self.map
            }
        }

        impl<'map> std::ops::Deref for $name<'map> {
            type Target = $data_ty;

            #[inline]
            fn deref(&self) -> &'map Self::Target {
                self.data
            }
        }
    };
}

pub(crate) use get_attrs;
pub(crate) use map_wrapper;
pub(crate) use parse_tag;

use crate::{Gid, MapTilesetGid};

/// Returns both the tileset and its index
pub(crate) fn get_tileset_for_gid(
    tilesets: &[MapTilesetGid],
    gid: Gid,
) -> Option<(usize, &MapTilesetGid)> {
    tilesets
        .iter()
        .enumerate()
        .rev()
        .find(|(_idx, ts)| ts.first_gid <= gid)
}

pub fn floor_div(a: i32, b: i32) -> i32 {
    let d = a / b;
    let r = a % b;

    if r == 0 {
        d
    } else {
        d - ((a < 0) ^ (b < 0)) as i32
    }
}
