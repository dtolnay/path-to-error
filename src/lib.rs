//! [![github]](https://github.com/dtolnay/path-to-error)&ensp;[![crates-io]](https://crates.io/crates/serde_path_to_error)&ensp;[![docs-rs]](https://docs.rs/serde_path_to_error)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white&logo=data:image/svg+xml;base64,PHN2ZyByb2xlPSJpbWciIHhtbG5zPSJodHRwOi8vd3d3LnczLm9yZy8yMDAwL3N2ZyIgdmlld0JveD0iMCAwIDUxMiA1MTIiPjxwYXRoIGZpbGw9IiNmNWY1ZjUiIGQ9Ik00ODguNiAyNTAuMkwzOTIgMjE0VjEwNS41YzAtMTUtOS4zLTI4LjQtMjMuNC0zMy43bC0xMDAtMzcuNWMtOC4xLTMuMS0xNy4xLTMuMS0yNS4zIDBsLTEwMCAzNy41Yy0xNC4xIDUuMy0yMy40IDE4LjctMjMuNCAzMy43VjIxNGwtOTYuNiAzNi4yQzkuMyAyNTUuNSAwIDI2OC45IDAgMjgzLjlWMzk0YzAgMTMuNiA3LjcgMjYuMSAxOS45IDMyLjJsMTAwIDUwYzEwLjEgNS4xIDIyLjEgNS4xIDMyLjIgMGwxMDMuOS01MiAxMDMuOSA1MmMxMC4xIDUuMSAyMi4xIDUuMSAzMi4yIDBsMTAwLTUwYzEyLjItNi4xIDE5LjktMTguNiAxOS45LTMyLjJWMjgzLjljMC0xNS05LjMtMjguNC0yMy40LTMzLjd6TTM1OCAyMTQuOGwtODUgMzEuOXYtNjguMmw4NS0zN3Y3My4zek0xNTQgMTA0LjFsMTAyLTM4LjIgMTAyIDM4LjJ2LjZsLTEwMiA0MS40LTEwMi00MS40di0uNnptODQgMjkxLjFsLTg1IDQyLjV2LTc5LjFsODUtMzguOHY3NS40em0wLTExMmwtMTAyIDQxLjQtMTAyLTQxLjR2LS42bDEwMi0zOC4yIDEwMiAzOC4ydi42em0yNDAgMTEybC04NSA0Mi41di03OS4xbDg1LTM4Ljh2NzUuNHptMC0xMTJsLTEwMiA0MS40LTEwMi00MS40di0uNmwxMDItMzguMiAxMDIgMzguMnYuNnoiPjwvcGF0aD48L3N2Zz4K
//!
//! <br>
//!
//! Find out the path at which a deserialization error occurred. This crate
//! provides a wrapper that works with any existing Serde `Deserializer` and
//! exposes the chain of field names leading to the error.
//!
//! # Example
//!
//! ```
//! # use serde_derive::Deserialize;
//! #
//! use serde::Deserialize;
//! use std::collections::BTreeMap as Map;
//!
//! #[derive(Deserialize)]
//! struct Package {
//!     name: String,
//!     dependencies: Map<String, Dependency>,
//! }
//!
//! #[derive(Deserialize)]
//! struct Dependency {
//!     version: String,
//! }
//!
//! fn main() {
//!     let j = r#"{
//!         "name": "demo",
//!         "dependencies": {
//!             "serde": {
//!                 "version": 1
//!             }
//!         }
//!     }"#;
//!
//!     // Some Deserializer.
//!     let jd = &mut serde_json::Deserializer::from_str(j);
//!
//!     let result: Result<Package, _> = serde_path_to_error::deserialize(jd);
//!     match result {
//!         Ok(_) => panic!("expected a type error"),
//!         Err(err) => {
//!             let path = err.path().to_string();
//!             assert_eq!(path, "dependencies.serde.version");
//!         }
//!     }
//! }
//! ```

#![doc(html_root_url = "https://docs.rs/serde_path_to_error/0.1.4")]

use serde::de::{self, Deserialize, DeserializeSeed, Visitor};
use serde::serde_if_integer128;
use std::error::Error as StdError;
use std::fmt::{self, Display};

mod path;
pub use crate::path::{Path, Segment, Segments};

/// Original deserializer error together with the path at which it occurred.
#[derive(Clone, Debug)]
pub struct Error<E> {
    path: Path,
    original: E,
}

impl<E> Error<E> {
    /// Element path at which this deserialization error occurred.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The Deserializer's underlying error that occurred.
    pub fn into_inner(self) -> E {
        self.original
    }

    /// Reference to the Deserializer's underlying error that occurred.
    pub fn inner(&self) -> &E {
        &self.original
    }
}

impl<E: Display> Display for Error<E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.path(), self.inner())
    }
}

impl<E: StdError + 'static> StdError for Error<E> {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.inner())
    }
}

/// State for bookkeeping across nested deserializer calls.
///
/// You don't need this if you are using `serde_path_to_error::deserializer`. If
/// you are managing your own `Deserializer`, see the usage example on
/// [`Deserializer`].
pub struct Track {
    path: Option<Path>,
}

impl Track {
    /// Empty state with no error having happened yet.
    pub fn new() -> Self {
        Track { path: None }
    }

    /// Gets path at which the error occurred. Only meaningful after we know
    /// that an error has occurred. Returns an empty path otherwise.
    pub fn path(self) -> Path {
        self.path.unwrap_or_else(Path::empty)
    }

    #[inline]
    fn trigger<E>(&mut self, chain: &Chain, err: E) -> E {
        self.trigger_impl(chain);
        err
    }

    fn trigger_impl(&mut self, chain: &Chain) {
        if self.path.is_none() {
            self.path = Some(Path::from_chain(chain));
        }
    }
}

/// Entry point. See [crate documentation][crate] for an example.
pub fn deserialize<'de, D, T>(deserializer: D) -> Result<T, Error<D::Error>>
where
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    let mut track = Track::new();
    match T::deserialize(Deserializer::new(deserializer, &mut track)) {
        Ok(t) => Ok(t),
        Err(err) => Err(Error {
            path: track.path(),
            original: err,
        }),
    }
}

/// Deserializer adapter that records path to deserialization errors.
///
/// # Example
///
/// ```
/// # use serde_derive::Deserialize;
/// #
/// use serde::Deserialize;
/// use std::collections::BTreeMap as Map;
///
/// #[derive(Deserialize)]
/// struct Package {
///     name: String,
///     dependencies: Map<String, Dependency>,
/// }
///
/// #[derive(Deserialize)]
/// struct Dependency {
///     version: String,
/// }
///
/// fn main() {
///     let j = r#"{
///         "name": "demo",
///         "dependencies": {
///             "serde": {
///                 "version": 1
///             }
///         }
///     }"#;
///
///     // Some Deserializer.
///     let jd = &mut serde_json::Deserializer::from_str(j);
///
///     let mut track = serde_path_to_error::Track::new();
///     let pd = serde_path_to_error::Deserializer::new(jd, &mut track);
///
///     match Package::deserialize(pd) {
///         Ok(_) => panic!("expected a type error"),
///         Err(_) => {
///             let path = track.path().to_string();
///             assert_eq!(path, "dependencies.serde.version");
///         }
///     }
/// }
/// ```
pub struct Deserializer<'a, 'b, D> {
    de: D,
    chain: Chain<'a>,
    track: &'b mut Track,
}

impl<'a, 'b, D> Deserializer<'a, 'b, D> {
    pub fn new(de: D, track: &'b mut Track) -> Self {
        Deserializer {
            de,
            chain: Chain::Root,
            track,
        }
    }
}

#[derive(Clone)]
enum Chain<'a> {
    Root,
    Seq {
        parent: &'a Chain<'a>,
        index: usize,
    },
    Map {
        parent: &'a Chain<'a>,
        key: String,
    },
    Enum {
        parent: &'a Chain<'a>,
        variant: String,
    },
    Some {
        parent: &'a Chain<'a>,
    },
    NewtypeStruct {
        parent: &'a Chain<'a>,
    },
    NewtypeVariant {
        parent: &'a Chain<'a>,
    },
    NonStringKey {
        parent: &'a Chain<'a>,
    },
}

// Plain old forwarding impl.
impl<'a, 'b, 'de, D> de::Deserializer<'de> for Deserializer<'a, 'b, D>
where
    D: de::Deserializer<'de>,
{
    type Error = D::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_any(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_bool(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_u8(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_u16(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_u32(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_u64(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_i8(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_i16(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_i32(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_i64(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    serde_if_integer128! {
        fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, D::Error>
        where
            V: Visitor<'de>,
        {
            let chain = self.chain;
            let track = self.track;
            self.de
                .deserialize_u128(Wrap::new(visitor, &chain, track))
                .map_err(|err| track.trigger(&chain, err))
        }

        fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, D::Error>
        where
            V: Visitor<'de>,
        {
            let chain = self.chain;
            let track = self.track;
            self.de
                .deserialize_i128(Wrap::new(visitor, &chain, track))
                .map_err(|err| track.trigger(&chain, err))
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_f32(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_f64(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_char(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_str(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_string(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_bytes(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_byte_buf(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_option(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_unit(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_unit_struct(name, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_newtype_struct(name, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_seq(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_tuple(len, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_tuple_struct(name, len, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_map(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_struct(name, fields, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_enum(name, variants, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_ignored_any(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, D::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.de
            .deserialize_identifier(Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }
}

// Wrapper that attaches context to a `Visitor`, `SeqAccess` or `EnumAccess`.
struct Wrap<'a, 'b, X> {
    delegate: X,
    chain: &'a Chain<'a>,
    track: &'b mut Track,
}

// Wrapper that attaches context to a `VariantAccess`.
struct WrapVariant<'a, 'b, X> {
    delegate: X,
    chain: Chain<'a>,
    track: &'b mut Track,
}

impl<'a, 'b, X> Wrap<'a, 'b, X> {
    fn new(delegate: X, chain: &'a Chain<'a>, track: &'b mut Track) -> Self {
        Wrap {
            delegate,
            chain,
            track,
        }
    }
}

impl<'a, 'b, X> WrapVariant<'a, 'b, X> {
    fn new(delegate: X, chain: Chain<'a>, track: &'b mut Track) -> Self {
        WrapVariant {
            delegate,
            chain,
            track,
        }
    }
}

// Forwarding impl to preserve context.
impl<'a, 'b, 'de, X> Visitor<'de> for Wrap<'a, 'b, X>
where
    X: Visitor<'de>,
{
    type Value = X::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.delegate.expecting(formatter)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_bool(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_i8(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_i16(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_i32(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_i64(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_u8(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_u16(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_u32(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_u64(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_f32(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_f64(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_char(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_str(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_borrowed_str(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_string(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_unit()
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_none()
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_some(Deserializer {
                de: deserializer,
                chain: Chain::Some { parent: chain },
                track: track,
            })
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_newtype_struct(Deserializer {
                de: deserializer,
                chain: Chain::NewtypeStruct { parent: chain },
                track: track,
            })
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_seq<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_seq(SeqAccess::new(visitor, chain, track))
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_map(MapAccess::new(visitor, chain, track))
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_enum<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::EnumAccess<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_enum(Wrap::new(visitor, chain, track))
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_bytes(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_borrowed_bytes(v)
            .map_err(|err| track.trigger(chain, err))
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .visit_byte_buf(v)
            .map_err(|err| track.trigger(chain, err))
    }
}

// Forwarding impl to preserve context.
impl<'a, 'b, 'de, X: 'a> de::EnumAccess<'de> for Wrap<'a, 'b, X>
where
    X: de::EnumAccess<'de>,
{
    type Error = X::Error;
    type Variant = WrapVariant<'a, 'b, X::Variant>;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), X::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        let mut variant = None;
        self.delegate
            .variant_seed(CaptureKey::new(seed, &mut variant))
            .map_err(|err| track.trigger(chain, err))
            .map(move |(v, vis)| {
                let chain = match variant {
                    Some(variant) => Chain::Enum {
                        parent: chain,
                        variant,
                    },
                    None => Chain::NonStringKey { parent: chain },
                };
                (v, WrapVariant::new(vis, chain, track))
            })
    }
}

// Forwarding impl to preserve context.
impl<'a, 'b, 'de, X> de::VariantAccess<'de> for WrapVariant<'a, 'b, X>
where
    X: de::VariantAccess<'de>,
{
    type Error = X::Error;

    fn unit_variant(self) -> Result<(), X::Error> {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .unit_variant()
            .map_err(|err| track.trigger(&chain, err))
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, X::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        let nested = Chain::NewtypeVariant { parent: &chain };
        self.delegate
            .newtype_variant_seed(TrackedSeed::new(seed, nested, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .tuple_variant(len, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }

    fn struct_variant<V>(
        self,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.delegate
            .struct_variant(fields, Wrap::new(visitor, &chain, track))
            .map_err(|err| track.trigger(&chain, err))
    }
}

// Seed that saves the string into the given optional during `visit_str` and
// `visit_string`.
struct CaptureKey<'a, X> {
    delegate: X,
    key: &'a mut Option<String>,
}

impl<'a, X> CaptureKey<'a, X> {
    fn new(delegate: X, key: &'a mut Option<String>) -> Self {
        CaptureKey {
            delegate: delegate,
            key: key,
        }
    }
}

// Forwarding impl.
impl<'a, 'de, X> DeserializeSeed<'de> for CaptureKey<'a, X>
where
    X: DeserializeSeed<'de>,
{
    type Value = X::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<X::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.delegate
            .deserialize(CaptureKey::new(deserializer, self.key))
    }
}

// Forwarding impl.
impl<'a, 'de, X> de::Deserializer<'de> for CaptureKey<'a, X>
where
    X: de::Deserializer<'de>,
{
    type Error = X::Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_any(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_bool(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_u8(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_u16(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_u32(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_u64(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_i8(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_i16(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_i32(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_i64(CaptureKey::new(visitor, self.key))
    }

    serde_if_integer128! {
        fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, X::Error>
        where
            V: Visitor<'de>,
        {
            self.delegate
                .deserialize_u128(CaptureKey::new(visitor, self.key))
        }

        fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, X::Error>
        where
            V: Visitor<'de>,
        {
            self.delegate
                .deserialize_i128(CaptureKey::new(visitor, self.key))
        }
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_f32(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_f64(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_char(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_str(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_string(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_bytes(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_byte_buf(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_option(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_unit(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_unit_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_unit_struct(name, CaptureKey::new(visitor, self.key))
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_newtype_struct(name, CaptureKey::new(visitor, self.key))
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_seq(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_tuple(len, CaptureKey::new(visitor, self.key))
    }

    fn deserialize_tuple_struct<V>(
        self,
        name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_tuple_struct(name, len, CaptureKey::new(visitor, self.key))
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_map(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_struct<V>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_struct(name, fields, CaptureKey::new(visitor, self.key))
    }

    fn deserialize_enum<V>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_enum(name, variants, CaptureKey::new(visitor, self.key))
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_ignored_any(CaptureKey::new(visitor, self.key))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, X::Error>
    where
        V: Visitor<'de>,
    {
        self.delegate
            .deserialize_identifier(CaptureKey::new(visitor, self.key))
    }
}

// Forwarding impl except `visit_str` and `visit_string` which save the string.
impl<'a, 'de, X> Visitor<'de> for CaptureKey<'a, X>
where
    X: Visitor<'de>,
{
    type Value = X::Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        self.delegate.expecting(formatter)
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_bool(v)
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_i8(v)
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_i16(v)
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_i32(v)
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_i64(v)
    }

    fn visit_u8<E>(self, v: u8) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_u8(v)
    }

    fn visit_u16<E>(self, v: u16) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_u16(v)
    }

    fn visit_u32<E>(self, v: u32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_u32(v)
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_u64(v)
    }

    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_f32(v)
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_f64(v)
    }

    fn visit_char<E>(self, v: char) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_char(v)
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        *self.key = Some(v.to_owned());
        self.delegate.visit_str(v)
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        *self.key = Some(v.to_owned());
        self.delegate.visit_borrowed_str(v)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        *self.key = Some(v.clone());
        self.delegate.visit_string(v)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_unit()
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_none()
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.delegate.visit_some(deserializer)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        self.delegate.visit_newtype_struct(deserializer)
    }

    fn visit_seq<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::SeqAccess<'de>,
    {
        self.delegate.visit_seq(visitor)
    }

    fn visit_map<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::MapAccess<'de>,
    {
        self.delegate.visit_map(visitor)
    }

    fn visit_enum<V>(self, visitor: V) -> Result<Self::Value, V::Error>
    where
        V: de::EnumAccess<'de>,
    {
        self.delegate.visit_enum(visitor)
    }

    fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_bytes(v)
    }

    fn visit_borrowed_bytes<E>(self, v: &'de [u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_borrowed_bytes(v)
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.delegate.visit_byte_buf(v)
    }
}

// Seed used for map values, sequence elements and newtype variants to track
// their path.
struct TrackedSeed<'a, 'b, X> {
    seed: X,
    chain: Chain<'a>,
    track: &'b mut Track,
}

impl<'a, 'b, X> TrackedSeed<'a, 'b, X> {
    fn new(seed: X, chain: Chain<'a>, track: &'b mut Track) -> Self {
        TrackedSeed { seed, chain, track }
    }
}

impl<'a, 'b, 'de, X> DeserializeSeed<'de> for TrackedSeed<'a, 'b, X>
where
    X: DeserializeSeed<'de>,
{
    type Value = X::Value;

    fn deserialize<D>(self, deserializer: D) -> Result<X::Value, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let chain = self.chain;
        let track = self.track;
        self.seed
            .deserialize(Deserializer {
                de: deserializer,
                chain: chain.clone(),
                track: track,
            })
            .map_err(|err| track.trigger(&chain, err))
    }
}

// Seq visitor that tracks the index of its elements.
struct SeqAccess<'a, 'b, X> {
    delegate: X,
    chain: &'a Chain<'a>,
    index: usize,
    track: &'b mut Track,
}

impl<'a, 'b, X> SeqAccess<'a, 'b, X> {
    fn new(delegate: X, chain: &'a Chain<'a>, track: &'b mut Track) -> Self {
        SeqAccess {
            delegate,
            chain,
            index: 0,
            track,
        }
    }
}

// Forwarding impl to preserve context.
impl<'a, 'b, 'de, X> de::SeqAccess<'de> for SeqAccess<'a, 'b, X>
where
    X: de::SeqAccess<'de>,
{
    type Error = X::Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, X::Error>
    where
        T: DeserializeSeed<'de>,
    {
        let parent = self.chain;
        let chain = Chain::Seq {
            parent,
            index: self.index,
        };
        let track = &mut *self.track;
        self.index += 1;
        self.delegate
            .next_element_seed(TrackedSeed::new(seed, chain, track))
            .map_err(|err| track.trigger(parent, err))
    }

    fn size_hint(&self) -> Option<usize> {
        self.delegate.size_hint()
    }
}

// Map visitor that captures the string value of its keys and uses that to track
// the path to its values.
struct MapAccess<'a, 'b, X> {
    delegate: X,
    chain: &'a Chain<'a>,
    key: Option<String>,
    track: &'b mut Track,
}

impl<'a, 'b, X> MapAccess<'a, 'b, X> {
    fn new(delegate: X, chain: &'a Chain<'a>, track: &'b mut Track) -> Self {
        MapAccess {
            delegate,
            chain,
            key: None,
            track,
        }
    }
}

impl<'a, 'b, 'de, X> de::MapAccess<'de> for MapAccess<'a, 'b, X>
where
    X: de::MapAccess<'de>,
{
    type Error = X::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, X::Error>
    where
        K: DeserializeSeed<'de>,
    {
        let chain = self.chain;
        let track = &mut *self.track;
        let key = &mut self.key;
        self.delegate
            .next_key_seed(CaptureKey::new(seed, key))
            .map_err(|err| {
                let chain = match key.take() {
                    Some(key) => Chain::Map { parent: chain, key },
                    None => Chain::NonStringKey { parent: chain },
                };
                track.trigger(&chain, err)
            })
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, X::Error>
    where
        V: DeserializeSeed<'de>,
    {
        let parent = self.chain;
        let chain = match self.key.take() {
            Some(key) => Chain::Map { parent, key },
            None => Chain::NonStringKey { parent },
        };
        let track = &mut *self.track;
        self.delegate
            .next_value_seed(TrackedSeed::new(seed, chain, track))
            .map_err(|err| track.trigger(parent, err))
    }

    fn size_hint(&self) -> Option<usize> {
        self.delegate.size_hint()
    }
}
