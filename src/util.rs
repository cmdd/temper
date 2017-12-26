//! Module `util.rs` provides internal, generic utility functions for the crate.

/// Given a Vec in sorted ascending order and a curren index, function `walk`
/// start from the current index and move upwards, returning the largest index
/// with the same value.
///
/// # Examples
///
/// ```
/// let v = vec![0, 1, 1, 2, 3, 5, 8, 8]
///
/// assert_eq!(walk(1, &v), 2);
/// assert_eq!(walk(7, &v), 7);
/// ```
pub fn walk<T: PartialEq>(ix: usize, v: &[T]) -> usize {
    let val = &v[ix];
    let max = v.len();

    for (i, vn) in v.iter().enumerate().skip(ix) {
        if vn != val {
            return i - 1;
        }
    }

    max - 1
}
