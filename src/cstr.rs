#[derive(Clone, Copy)]
pub struct CStr<'a> {
    bytes: &'a [u8],
}

impl<'a> CStr<'a> {
    /// Requires that the passed-in pointer be a valid pointer to a null-terminated array of c_char
    pub unsafe fn from_ptr(ptr: *const u8) -> CStr<'a> {
        CStr {
            bytes: core::slice::from_raw_parts(ptr, libc::strlen(ptr as *const i8) + 1),
        }
    }

    pub fn from_bytes(bytes: &'a [u8]) -> CStr<'a> {
        assert!(
            bytes.last() == Some(&0),
            "attempted to construct a CStr from a slice without a null terminator"
        );
        CStr { bytes }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes[..self.bytes.len() - 1]
    }

    pub fn get(&self, i: usize) -> Option<&u8> {
        self.bytes[..self.bytes.len() - 1].get(i)
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.bytes.as_ptr()
    }

    pub fn len(&self) -> usize {
        self.bytes.len() - 1
    }
}

trait SliceExt {
    fn digit_at(&self, index: usize) -> bool;
}

impl SliceExt for &[u8] {
    fn digit_at(&self, index: usize) -> bool {
        self.get(index).map(u8::is_ascii_digit).unwrap_or(false)
    }
}

impl<'a> PartialEq<&[u8]> for CStr<'a> {
    fn eq(&self, bytes: &&[u8]) -> bool {
        if bytes.last() == Some(&0) {
            self.bytes == *bytes
        } else {
            &self.bytes[..self.bytes.len() - 1] == *bytes
        }
    }
}
