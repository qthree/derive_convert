use derive_convert::Convert;

#[derive(Convert)]
#[from(V1 = "Color1")]
enum Color {
    Red,
    #[from(V1(rename("Blues")))]
    Blue,
    #[from(V1(skip))]
    Green,
}

#[allow(dead_code)]
enum Color1 {
    Red,
    Blues,
}

#[derive(Convert)]
#[from_self(COMMON = "Color")]
#[allow(dead_code)]
enum Color2 {
    Red,
    Blue,
    Green,
}

#[derive(Convert)]
#[from(V1("Foo1", ignore("_c")))]
#[allow(dead_code)]
struct Foo {
    a: i32,
    b: String,
    #[from(V1(default))]
    d: f64,
}

struct Foo1 {
    a: i32,
    b: String,
    _c: (),
}

#[derive(Convert)]
#[from_self(COMMON = "Foo")]
struct Foo2 {
    a: i32,
    b: String,
    d: f64,
}
