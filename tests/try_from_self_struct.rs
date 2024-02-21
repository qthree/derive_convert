use std::{convert::Infallible, num::TryFromIntError};

use derive_convert::Convert;

#[derive(Convert, PartialEq, Debug)]
#[try_from_self(V1 = "Rect1", V2 = "v2::Rect2", Error = "Error")]
struct Rect {
    #[try_from_self(V2(skip))]
    tag: String,
    x: i32,
    #[try_from_self(V1(try_map = "try_some"), V2(skip))]
    y: Option<i32>,
    z: i32,
    #[try_from_self(map = "|a: Foo<_>| a.0")]
    width: Foo<i32>,
    #[try_from_self(try_map = "try_some", rename(Height))]
    height: Option<i32>,
    #[try_from_self(
        V1(
            try_map = "|vec: Vec<i64>| vec.into_iter().map(|val| val.try_into()).collect::<Result<Vec<i32>, _>>()"
        ),
        V2(skip)
    )]
    colors: Vec<i64>,
}

fn try_some<T>(opt: Option<T>) -> Result<T, Error> {
    opt.ok_or(Error::Missing)
}

#[derive(PartialEq, Debug)]
struct Foo<T>(T);

#[allow(dead_code)]
#[derive(PartialEq, Debug)]
#[allow(non_snake_case)]
struct Rect1 {
    tag: String,
    x: i32,
    y: i32,
    z: i64,
    width: i32,
    Height: i32,
    colors: Vec<i32>,
}

#[allow(dead_code)]
mod v2 {
    #[derive(PartialEq, Debug)]
    #[allow(non_snake_case)]
    pub(super) struct Rect2 {
        pub(super) x: i32,
        pub(super) z: i128,
        pub(super) width: i32,
        pub(super) Height: i32,
    }
}

#[derive(Debug)]
enum Error {
    TryFromInt,
    Missing,
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

#[test]
fn try_from_self_struct_1() {
    let rect = Rect {
        tag: "foo".into(),
        x: 1,
        y: Some(2),
        z: 3,
        width: Foo(4),
        height: Some(5),
        colors: vec![6, 7, 8],
    };
    let rect1 = Rect1 {
        tag: "foo".into(),
        x: 1,
        y: 2,
        z: 3,
        width: 4,
        Height: 5,
        colors: vec![6, 7, 8],
    };
    assert_eq!(Rect1::try_from(rect).unwrap(), rect1);
}

#[test]
fn try_from_self_struct_2() {
    let rect = Rect {
        tag: "".into(),
        x: 1,
        y: None,
        z: 3,
        width: Foo(4),
        height: Some(5),
        colors: vec![],
    };
    let rect2 = v2::Rect2 {
        x: 1,
        z: 3,
        width: 4,
        Height: 5,
    };
    assert_eq!(v2::Rect2::try_from(rect).unwrap(), rect2);
}
