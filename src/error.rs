pub struct Error(pub isize);

impl std::fmt::Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Error code: {}", self.0)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        writeln!(f, "Error code: {}", self.0)
    }
}

impl From<i32> for Error {
    fn from(e: i32) -> Error {
        Error(e as isize)
    }
}

impl From<isize> for Error {
    fn from(e: isize) -> Error {
        Error(e)
    }
}
