use std::{num::TryFromIntError, convert::Infallible};

use derive_try_from::TryFrom;

#[derive(TryFrom, PartialEq, Debug)]
#[try_from(V1 = "Rect1", V2 = "v2::Rect2", Error = "Error")]
struct Rect {
    #[try_from(V2(default))]
    tag: String,
    x: i32,
    #[try_from(V2(default))]
    y: Option<i32>,
    z: i32,
    #[try_from(with = "ok_foo")]
    width: Foo<i32>,
    height: Option<i32>,
    #[try_from(V1 = "|vec: Vec<i64>| vec.into_iter().map(|val| val.try_into()).collect::<Result<Vec<i32>, _>>()", V2(default))]
    colors: Vec<i32>,
}

#[derive(PartialEq, Debug)]
struct Foo<T>(T);
fn ok_foo<T>(foo: T) -> Result<Foo<T>, Error> {
    Ok(Foo(foo))
}

struct Rect1 {
    tag: String,
    x: i32,
    y: i32,
    z: i64,
    width: i32,
    height: i32,
    colors: Vec<i64>,
    _other1: i32,
}

mod v2 {
    pub(super) struct Rect2 {
        pub(super) x: i32,
        pub(super) z: i128,
        pub(super) width: i32,
        pub(super) height: i32,
        pub(super) _other2: i32,
    }
}

enum Error {
    TryFromInt,
}


impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Self {
        Self::TryFromInt
    }
}
