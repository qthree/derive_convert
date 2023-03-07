use derive_try_from::TryFrom;

#[derive(TryFrom, PartialEq, Debug)]
#[try_from(V1 = "Color1", V2 = "v2::Color2", Error = "()")]
enum Color {
    Red,
    Blue,
}

#[allow(dead_code)]
enum Color1 {
    Red,
    Blue,
}

mod v2 {
    #[allow(dead_code)]
    pub(super) enum Color2 {
        Red,
        Blue,
    }
}

#[test]
fn test_fry_from() {
    let color1: Color = Color1::Red.try_into().unwrap();
    assert_eq!(color1, Color::Red);
    let color2: Color = v2::Color2::Blue.try_into().unwrap();
    assert_eq!(color2, Color::Blue);
}