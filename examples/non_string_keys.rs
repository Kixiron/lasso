use lasso::Rodeo;

fn main() {
    // Create a rodeo that interns Vec<i32> instead of String
    let mut rodeo: Rodeo<Vec<i32>> = Rodeo::new();

    // Intern some integer sequences
    let a = rodeo.get_or_intern(vec![1, 2, 3]);
    let b = rodeo.get_or_intern(vec![4, 5, 6, 7, 8]);

    // Interning the same value returns the same key
    let a2 = rodeo.get_or_intern(vec![1, 2, 3]);
    assert_eq!(a, a2);

    // Resolve keys back to values
    assert_eq!(rodeo.resolve(&a), &[1, 2, 3]);
    assert_eq!(rodeo.resolve(&b), &[4, 5, 6, 7, 8]);

    // Lookup by value
    assert_eq!(rodeo.get([1, 2, 3].as_slice()), Some(a));
    assert_eq!(rodeo.get([7, 8, 9].as_slice()), None);

    println!("Interned {} sequences", rodeo.len());
}
