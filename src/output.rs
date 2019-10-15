use std::sync::{Arc, Mutex};
use veneer::{syscalls, Error};

lazy_static::lazy_static! {
    pub static ref LOCK: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
}

pub fn write_to_stdout(bytes: &[u8]) -> Result<(), Error> {
    if !bytes.is_empty() {
        let _handle = LOCK.lock().unwrap();
        let mut bytes_written = 0;
        while bytes_written < bytes.len() {
            bytes_written += syscalls::write(1, &bytes[bytes_written..])?;
        }
    }
    Ok(())
}
