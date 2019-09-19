use crate::syscalls;
use crate::Error;

pub struct BufferedStdout {
    buf: arrayvec::ArrayVec<[u8; 4096]>,
}

impl BufferedStdout {
    pub fn new() -> Self {
        Self {
            buf: arrayvec::ArrayVec::new(),
        }
    }

    pub fn write(&mut self, item: &[u8]) -> Result<(), Error> {
        for b in item {
            if self.buf.try_push(*b).is_err() {
                write_to_stdout(&self.buf)?;
                self.buf.clear();
                self.buf.push(*b);
            }
        }
        Ok(())
    }

    pub fn push(&mut self, b: u8) -> Result<(), Error> {
        if self.buf.try_push(b).is_err() {
            write_to_stdout(&self.buf)?;
            self.buf.clear();
            self.buf.push(b);
        }
        Ok(())
    }
}

impl Drop for BufferedStdout {
    fn drop(&mut self) {
        let _ = write_to_stdout(&self.buf);
    }
}

fn write_to_stdout(bytes: &[u8]) -> Result<(), Error> {
    let mut bytes_written = 0;
    while bytes_written < bytes.len() {
        bytes_written += syscalls::write(libc::STDOUT_FILENO, &bytes[bytes_written..])?;
    }
    Ok(())
}
