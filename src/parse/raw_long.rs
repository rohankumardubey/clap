use std::ffi::OsStr;

use parse::{RawValue, RawArg};
use util::OsStrExt2;

pub struct RawLong<'a> {
    // --foo
    long: &'a OsStr,
    value: Option<RawValue<'a>>,
}

impl<'a> RawLong<'a> {
    pub(crate) fn key_as_bytes(&self) -> &[u8] {
        self.long.as_bytes()
    }

    pub(crate) fn key(&self) -> &OsStr {
        self.long.trim_left_matches(b'-')
    }
}

impl<'a> From<RawArg<'a>> for RawLong<'a> {
    fn from(oss: RawArg<'a>) -> Self {
        let had_eq = oss.contains_byte(b'=');
        debug!("Parser::parse_long_arg: Does it contain '='...");
        if had_eq {
            sdebugln!("Yes '{:?}'", p1);
            let (p0, p1) = oss.split_at_byte(b'=');
            let trimmed = p1.trim_left_matches(b'=');
            RawLong {
                long: p0,
                value: Some(RawValue::from_trimmed(trimmed))
            }
        } else {
            sdebugln!("No");
            RawLong {
                long: oss.0,
                value: None
            }
        }
    }
}

