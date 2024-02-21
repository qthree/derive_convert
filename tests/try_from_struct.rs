use std::{convert::Infallible, num::TryFromIntError};

use derive_convert::Convert;

#[derive(Convert, PartialEq, Debug)]
#[try_from(
    V1("Rect1", ignore("_other1")),
    V2("v2::Rect2", ignore("_other2")),
    Error = "Error"
)]
struct Rect {
    #[try_from(V2(new = "String::new"))]
    tag: String,
    x: i32,
    #[try_from(V2(default))]
    y: Option<i32>,
    z: i32,
    #[try_from(map = "Foo")]
    width: Foo<i32>,
    #[try_from(V2(rename("Height")))]
    height: Option<i32>,
    #[try_from(
        V1(
            try_map = "|vec: Vec<i64>| vec.into_iter().map(|val| val.try_into()).collect::<Result<Vec<i32>, _>>()"
        ),
        V2(default)
    )]
    colors: Vec<i32>,
}

#[derive(PartialEq, Debug)]
struct Foo<T>(T);

#[derive(PartialEq, Debug)]
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
    #[derive(PartialEq, Debug)]
    #[allow(non_snake_case)]
    pub(super) struct Rect2 {
        pub(super) x: i32,
        pub(super) z: i128,
        pub(super) width: i32,
        pub(super) Height: i32,
        pub(super) _other2: i32,
    }
}

#[derive(Debug)]
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

#[test]
fn try_from_struct_1() {
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
        height: 5,
        colors: vec![6, 7, 8],
        _other1: 9,
    };
    assert_eq!(rect, rect1.try_into().unwrap());
}

#[test]
fn try_from_struct_2() {
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
        _other2: 9,
    };
    assert_eq!(rect, rect2.try_into().unwrap());
}
