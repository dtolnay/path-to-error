#![allow(dead_code)]

use serde::ser::{Serialize, SerializeMap as _, Serializer};
use serde_derive::{Deserialize, Serialize};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::fmt::{self, Display};

struct Counting;

thread_local! {
    static ALLOCS: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCS.with(|n| n.set(n.get() + 1));
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCS.with(|n| n.set(n.get() + 1));
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn allocations(f: impl FnOnce()) -> usize {
    let before = ALLOCS.with(Cell::get);
    f();
    ALLOCS.with(Cell::get) - before
}

#[derive(Deserialize, Serialize, Debug)]
struct Outer {
    id: u32,
    inner: Inner,
    kind: Kind,
    flags: [bool; 2],
}

#[derive(Deserialize, Serialize, Debug)]
struct Inner {
    enabled: bool,
}

#[derive(Deserialize, Serialize, Debug)]
enum Kind {
    Plain { count: u32 },
}

const JSON: &str =
    r#"{"id":1,"inner":{"enabled":true},"kind":{"Plain":{"count":2}},"flags":[true,false]}"#;

#[test]
fn deserialize_success_does_not_allocate() {
    let n = allocations(|| {
        let de = &mut serde_json::Deserializer::from_str(JSON);
        let _: Outer = serde_path_to_error::deserialize(de).unwrap();
    });
    assert_eq!(n, 0);
}

#[derive(Serialize)]
struct WithMap {
    outer: Outer,
    map: BTreeMap<&'static str, BTreeMap<u32, bool>>,
}

#[test]
fn serialize_success_does_not_allocate() {
    let value = WithMap {
        outer: serde_json::from_str(JSON).unwrap(),
        map: BTreeMap::from([("key", BTreeMap::from([(7, true)]))]),
    };
    let mut out = Vec::with_capacity(1024);
    let n = allocations(|| {
        let ser = &mut serde_json::Serializer::new(&mut out);
        serde_path_to_error::serialize(&value, ser).unwrap();
    });
    assert_eq!(n, 0);
}

#[test]
fn long_keys_still_report_the_path() {
    #[derive(Deserialize, Debug)]
    struct Long {
        a_field_name_that_is_longer_than_thirty_bytes: Inner,
    }

    let json = r#"{"a_field_name_that_is_longer_than_thirty_bytes":{"enabled":1}}"#;
    let de = &mut serde_json::Deserializer::from_str(json);
    let err = serde_path_to_error::deserialize::<_, Long>(de).unwrap_err();
    assert_eq!(
        err.path().to_string(),
        "a_field_name_that_is_longer_than_thirty_bytes.enabled",
    );
}

#[test]
fn long_display_keys_still_report_the_path() {
    struct Pieces;

    impl Display for Pieces {
        fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            for _ in 0..4 {
                formatter.write_str("0123456789")?;
            }
            Ok(())
        }
    }

    impl Serialize for Pieces {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(self)
        }
    }

    struct Map<'a>(&'a RefCell<String>);

    impl Serialize for Map<'_> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut map = serializer.serialize_map(Some(1))?;
            map.serialize_entry(&Pieces, self.0)?;
            map.end()
        }
    }

    let refcell = RefCell::new(String::new());
    let _borrowed = refcell.borrow_mut();
    let ser = &mut serde_json::Serializer::new(Vec::new());
    let err = serde_path_to_error::serialize(&Map(&refcell), ser).unwrap_err();
    assert_eq!(
        err.path().to_string(),
        "0123456789012345678901234567890123456789",
    );
}
