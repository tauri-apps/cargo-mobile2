use yes_or_no::yes_or_no;

yes_or_no!(Hungry);
yes_or_no!(HungrySerde, serde::Deserialize, serde::Serialize);

fn main() {
    assert_eq!(Hungry::from(true), Hungry::Yes);
    assert_eq!(Hungry::from(false), Hungry::No);
    assert!(Hungry::Yes.yes());
    assert!(Hungry::No.no());

    assert_eq!(Hungry::Yes & true, true);
    assert_eq!(true & Hungry::Yes, true);

    assert_eq!(Hungry::No | true, true);
    assert_eq!(false | Hungry::Yes, true);

    assert_eq!(Hungry::Yes ^ true, false);
    assert_eq!(false ^ Hungry::No, false);
}
