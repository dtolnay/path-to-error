use std::fmt::{self, Display};
use std::slice;

use super::Chain;

/// Path to the error value in the input, like `dependencies.serde.typo1`.
pub struct Path {
    segments: Vec<Segment>,
}

pub enum Segment {
    Seq { index: usize },
    Map { key: String },
    Option,
    Other,
}

impl Path {
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

        let mut needs_separator = false;
        for segment in self {
            if needs_separator {
                formatter.write_str(".")?;
            }
            match segment {
                Segment::Seq { index } => {
                    Display::fmt(index, formatter)?;
                    needs_separator = true;
                }
                Segment::Map { key } => {
                    Display::fmt(key, formatter)?;
                    needs_separator = true;
                }
                Segment::Option => {
                    needs_separator = false;
                }
                Segment::Other => {
                    formatter.write_str("?")?;
                    needs_separator = true;
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
                Chain::Some { parent } => {
                    segments.push(Segment::Option);
                    chain = parent;
                }
                Chain::NewtypeStruct { parent } | Chain::NewtypeVariant { parent } => {
                    segments.push(Segment::Other);
                    chain = parent;
                }
            }
        }
        segments.reverse();
        Path { segments }
    }
}
