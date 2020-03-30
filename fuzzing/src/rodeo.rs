use honggfuzz::fuzz;
use lasso::Rodeo;

fn main() {
    let mut rodeo = Rodeo::default();

    loop {
        fuzz!(|data: &[u8]| {
            if let Ok(string) = std::str::from_utf8(data) {
                if let Some(key) = rodeo.try_get_or_intern(string) {
                    assert_eq!(string, rodeo.resolve(&key));
                    assert_eq!(Some(key), rodeo.get(string));
                }
            } else {
                rodeo = Rodeo::default();
            }
        });
    }
}
