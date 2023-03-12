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
