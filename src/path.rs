use std::fmt::{self, Display};
use std::slice;

use super::Chain;

/// Path to the error value in the input, like `dependencies.serde.typo1`.
///
/// Use `path.to_string()` to get a string representation of the path with
/// segments separated by periods, or use `path.iter()` to iterate over
/// individual segments of the path.
pub struct Path {
    segments: Vec<Segment>,
}

/// Single segment of a path.
pub enum Segment {
    Seq { index: usize },
    Map { key: String },
    Enum { variant: String },
    Unknown,
}

impl Path {
    /// Returns an iterator with element type [`&Segment`][Segment].
    pub fn iter(&self) -> Segments {
        Segments {
            iter: self.segments.iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Path {
    type Item = &'a Segment;
    type IntoIter = Segments<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Iterator over segments of a path.
pub struct Segments<'a> {
    iter: slice::Iter<'a, Segment>,
}

impl<'a> Iterator for Segments<'a> {
    type Item = &'a Segment;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}

impl Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        if self.segments.is_empty() {
            return formatter.write_str(".");
        }

        for (i, segment) in self.iter().enumerate() {
            if i > 0 {
                formatter.write_str(".")?;
            }
            match segment {
                Segment::Seq { index } => {
                    Display::fmt(index, formatter)?;
                }
                Segment::Map { key } => {
                    Display::fmt(key, formatter)?;
                }
                Segment::Enum { variant } => {
                    Display::fmt(variant, formatter)?;
                }
                Segment::Unknown => {
                    formatter.write_str("?")?;
                }
            }
        }

        Ok(())
    }
}

impl Path {
    pub(crate) fn empty() -> Self {
        Path {
            segments: Vec::new(),
        }
    }

    pub(crate) fn from_chain(mut chain: &Chain) -> Self {
        let mut segments = Vec::new();
        loop {
            match chain {
                Chain::Root => break,
                Chain::Seq { parent, index } => {
                    segments.push(Segment::Seq { index: *index });
                    chain = parent;
                }
                Chain::Map { parent, key } => {
                    segments.push(Segment::Map { key: key.clone() });
                    chain = parent;
                }
                Chain::Enum { parent, variant } => {
                    segments.push(Segment::Enum {
                        variant: variant.clone(),
                    });
                    chain = parent;
                }
                Chain::Some { parent }
                | Chain::NewtypeStruct { parent }
                | Chain::NewtypeVariant { parent } => {
                    chain = parent;
                }
                Chain::NonStringKey { parent } => {
                    segments.push(Segment::Unknown);
                    chain = parent;
                }
            }
        }
        segments.reverse();
        Path { segments }
    }
}
