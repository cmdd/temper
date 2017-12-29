//! Module `util.rs` provides internal, generic utility functions for the crate.

use bytecount;

pub fn bind<A, B, F>(a: Result<A, B>, b: Result<A, B>, f: F) -> Result<A, B>
where
    F: Fn(A, A) -> A,
{
    match (a, b) {
        (Ok(va), Ok(vb)) => Ok(f(va, vb)),
        (Err(a), _) => Err(a),
        (_, Err(b)) => Err(b),
    }
}

#[inline(never)]
pub fn lines(buf: &[u8], eol: u8) -> usize {
    bytecount::count(buf, eol) + 1
}
