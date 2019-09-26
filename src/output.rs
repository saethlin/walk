use crate::syscalls;
use crate::Error;
use std::sync::{Arc, Mutex};

lazy_static::lazy_static! {
    pub static ref LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

pub struct BufferedStdout {
    buf: Vec<u8>,
}

impl BufferedStdout {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(4096),
        }
    }

    pub fn write(&mut self, item: &[u8]) -> &mut Self {
        self.buf.extend(item);
        self.check_flush();
        self
    }

    pub fn push(&mut self, b: u8) -> &mut Self {
        self.buf.push(b);
        self.check_flush();
        self
    }

    fn check_flush(&mut self) {
        if self.buf.len() > 2048 && self.buf.last() == Some(&b'\n') {
            write_to_stdout(&self.buf).unwrap();
            self.buf.clear();
        }
    }
}

impl Drop for BufferedStdout {
    fn drop(&mut self) {
        let _ = write_to_stdout(&self.buf);
    }
}

fn write_to_stdout(bytes: &[u8]) -> Result<(), Error> {
    if !bytes.is_empty() {
        let _handle = LOCK.lock().unwrap();
        let mut bytes_written = 0;
        while bytes_written < bytes.len() {
            bytes_written += syscalls::write(libc::STDOUT_FILENO, &bytes[bytes_written..])?;
        }
    }
    Ok(())
}
