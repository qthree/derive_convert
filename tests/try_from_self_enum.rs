use derive_convert::Convert;

#[derive(Convert, PartialEq, Debug)]
#[try_from_self(V1 = "Color1", V2 = "v2::Color2", Error = "()")]
enum Color {
    Red,
    #[try_from_self(V1(rename("Blues")))]
    Blue,
}

#[allow(dead_code)]
#[derive(PartialEq, Debug)]
enum Color1 {
    Red,
    Blues,
}

mod v2 {
    #[allow(dead_code)]
    #[derive(PartialEq, Debug)]
    pub(super) enum Color2 {
        Red,
        Blue,
    }
}

#[test]
fn try_from_self_enum() {
    let color1: Color1 = Color::Red.try_into().unwrap();
    assert_eq!(color1, Color1::Red);
    let color2: v2::Color2 = Color::Blue.try_into().unwrap();
    assert_eq!(color2, v2::Color2::Blue);
}
