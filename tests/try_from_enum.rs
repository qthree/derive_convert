use derive_convert::Convert;

#[derive(Convert, PartialEq, Debug)]
#[try_from(V1 = "Color1", V2 = "v2::Color2", Error = "()")]
enum Color {
    Red,
    #[try_from(V1(rename("Blues")))]
    Blue,
    #[try_from(V1(skip))]
    Green,
    #[try_from(skip)]
    #[allow(dead_code)]
    Black,
}

#[allow(dead_code)]
enum Color1 {
    Red,
    Blues,
}

mod v2 {
    #[allow(dead_code)]
    pub(super) enum Color2 {
        Red,
        Blue,
        Green,
    }
}

#[test]
fn try_from_enum() {
    let color1: Color = Color1::Red.try_into().unwrap();
    assert_eq!(color1, Color::Red);
    let color2: Color = v2::Color2::Blue.try_into().unwrap();
    assert_eq!(color2, Color::Blue);
}
