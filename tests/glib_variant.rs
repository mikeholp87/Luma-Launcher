use glib::prelude::*;

#[test]
fn variant_string_iterator_supports_both_directions() {
    let variant = vec!["first", "middle", "last"].to_variant();
    let mut iter = variant.array_iter_str().unwrap();
    assert_eq!(iter.next(), Some("first"));
    assert_eq!(iter.next_back(), Some("last"));
    assert_eq!(iter.next(), Some("middle"));
    assert_eq!(iter.next(), None);
    assert_eq!(variant.array_iter_str().unwrap().nth(1), Some("middle"));
    assert_eq!(variant.array_iter_str().unwrap().last(), Some("last"));
}
