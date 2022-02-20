use crate::bagit::consts::*;
use std::borrow::Cow;

const CR_ENCODED: &str = "%0D";
const LF_ENCODED: &str = "%0A";
const PERCENT_ENCODED: &str = "%25";

/// Percent encodes any CR, LF, or % characters in the input string
pub fn percent_encode(value: &str) -> Cow<str> {
    if let Some(i) = value.find(|c: char| c == CR || c == LF || c == '%') {
        let mut encoded = Vec::with_capacity(value.len() + 2);
        encoded.extend_from_slice(value[..i].as_bytes());

        let search = value[i..].bytes();

        for c in search {
            match c {
                CR_B => encoded.extend_from_slice(CR_ENCODED.as_bytes()),
                LF_B => encoded.extend_from_slice(LF_ENCODED.as_bytes()),
                b'%' => encoded.extend_from_slice(PERCENT_ENCODED.as_bytes()),
                _ => encoded.push(c),
            }
        }

        // This is fine because the original value is known to be valid UTF-8
        Cow::Owned(unsafe { String::from_utf8_unchecked(encoded) })
    } else {
        value.into()
    }
}

#[cfg(test)]
mod tests {
    use crate::bagit::encoding::percent_encode;

    #[test]
    fn test_percent_encoding() {
        assert_eq!(
            "a\tbc%25123%0Dqwe%0A%25%25asd%0D%0A !",
            percent_encode("a\tbc%123\rqwe\n%%asd\r\n !")
        );
        assert_eq!("nothing to see here", percent_encode("nothing to see here"));
    }
}
