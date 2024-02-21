use std::convert::Infallible;

use derive_convert::Convert;

pub fn ref_try_into_opt<'a, F, T>(
    value: &'a Option<F>,
) -> Result<Option<T>, <&'a F as TryInto<T>>::Error>
where
    &'a F: TryInto<T>,
{
    match value {
        Some(value) => Ok(Some(value.try_into()?)),
        None => Ok(None),
    }
}

#[derive(Convert, Debug)]
#[try_from(V1("&Foo1"), Error = "Error")]
pub struct Foo {
    #[try_from(try_map_ref = "ref_try_into_opt")]
    pub bar: Option<String>,
}

#[derive(Debug)]
struct Foo1 {
    bar: Option<Bar>,
}

#[derive(Debug)]
struct Bar(String);

impl TryFrom<&Bar> for String {
    type Error = Error;

    fn try_from(value: &Bar) -> Result<Self, Self::Error> {
        Ok(value.0.clone())
    }
}

#[derive(Debug)]
enum Error {
}

impl From<Infallible> for Error {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}
