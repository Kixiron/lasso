use lasso::{Rodeo, Spur};

#[test]
fn strings() {
    let string = "some random strings I've got";
    let mut rodeo: Rodeo<str, Spur> = Rodeo::new();

    let key1 = rodeo.get_or_intern(string);
    let key2 = rodeo.get_or_intern(string);

    assert_eq!(key1, key2);
    assert_eq!(rodeo.resolve(&key1), "some random strings I've got");
    assert_eq!(rodeo.resolve(&key2), "some random strings I've got");

    let string = "some random strings I've got".to_string();
    let mut rodeo: Rodeo<str, Spur> = Rodeo::new();

    let key1 = rodeo.get_or_intern(string.clone());
    let key2 = rodeo.get_or_intern(string);

    assert_eq!(key1, key2);
    assert_eq!(rodeo.resolve(&key1), "some random strings I've got");
    assert_eq!(rodeo.resolve(&key2), "some random strings I've got");
}

#[test]
fn bytes() {
    let bytes = b"some random bytes I've got";
    let mut rodeo: Rodeo<[u8], Spur> = Rodeo::new();

    let key1 = rodeo.get_or_intern(bytes);
    let key2 = rodeo.get_or_intern(bytes);

    assert_eq!(key1, key2);
    assert_eq!(rodeo.resolve(&key1), b"some random bytes I've got");
    assert_eq!(rodeo.resolve(&key2), b"some random bytes I've got");
}

#[test]
#[cfg(not(feature = "no-std"))]
fn cstr() {
    use std::ffi::{CStr, CString};

    let string = CString::new("some random strings I've got").unwrap();
    let mut rodeo: Rodeo<CStr, Spur> = Rodeo::new();

    let key1 = rodeo.get_or_intern(&string);
    let key2 = rodeo.get_or_intern(string.clone());
    let key3 = rodeo.get_or_intern(string);

    assert_eq!(key1, key2);
    assert_eq!(key1, key3);
    assert_eq!(key2, key3);
    assert_eq!(
        rodeo.resolve(&key1),
        CStr::from_bytes_with_nul(b"some random strings I've got\0").unwrap(),
    );
    assert_eq!(
        rodeo.resolve(&key2),
        CStr::from_bytes_with_nul(b"some random strings I've got\0").unwrap(),
    );
    assert_eq!(
        rodeo.resolve(&key3),
        CStr::from_bytes_with_nul(b"some random strings I've got\0").unwrap(),
    );
}
