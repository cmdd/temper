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

#[cfg(test)]
mod tests {
    use super::*;

    const LF: &'static str =
        "\
         This is some wonderful multi-line text to test to see if `lines` can \n\
         correctly determine the number of lines in a string. This str uses LF \n\
         line endings.";

    const CRLF: &'static str =
        "\
         This is some wonderful multi-line text to test to see if `lines` can \r\n\
         correctly determine the number of lines in a string. This str uses \r\n\
         CRLF line endings.";

    const LF_EMPTY: &'static str = "\n\n";

    const CRLF_EMPTY: &'static str = "\r\n\r\n";

    // TODO: Quickcheck?
    #[test]
    fn bind_add_oks() {
        let a: Result<u8, &str> = Ok(5);
        let b: Result<u8, &str> = Ok(10);
        assert_eq!(Ok(15), bind(a, b, |a, b| a + b));
    }

    #[test]
    fn bind_add_left_err() {
        let a: Result<u8, &str> = Err("Left error");
        let b: Result<u8, &str> = Ok(10);
        assert_eq!(a, bind(a, b, |a, b| a + b));
    }

    #[test]
    fn bind_add_right_err() {
        let a: Result<u8, &str> = Ok(5);
        let b: Result<u8, &str> = Err("Right error");
        assert_eq!(b, bind(a, b, |a, b| a + b));
    }

    #[test]
    fn lines_lf() {
        assert_eq!(3, lines(LF.as_bytes(), b'\n'));
    }

    #[test]
    fn lines_crlf() {
        assert_eq!(3, lines(CRLF.as_bytes(), b'\n'));
    }

    #[test]
    fn lines_lf_empty() {
        assert_eq!(3, lines(LF_EMPTY.as_bytes(), b'\n'));
    }

    #[test]
    fn lines_crlf_empty() {
        assert_eq!(3, lines(CRLF_EMPTY.as_bytes(), b'\n'));
    }
}
