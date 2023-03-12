use derive_convert::Convert;

#[derive(Convert)]
#[from(V1 = "Color1")]
enum Color {
    Red,
    Blue,
}

#[allow(dead_code)]
enum Color1 {
    Red,
    Blue,
}

#[derive(Convert)]
#[from_self(COMMON = "Color")]
#[allow(dead_code)]
enum Color2 {
    Red,
    Blue,
}

#[derive(Convert)]
#[from(V1 = "Foo1")]
#[allow(dead_code)]
struct Foo {
    a: i32,
    b: String,
}

struct Foo1 {
    a: i32,
    b: String,
}

#[derive(Convert)]
#[from_self(COMMON = "Foo")]
struct Foo2 {
    a: i32,
    b: String,
}
