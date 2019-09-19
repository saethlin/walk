use crate::syscalls;
use crate::{CStr, Error};
use libc::c_int;

pub struct Directory {
    fd: c_int,
    dirents: Vec<u8>,
    bytes_used: isize,
}

impl Directory {
    pub fn raw_fd(&self) -> c_int {
        self.fd
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = syscalls::close(self.fd);
    }
}

impl<'a> Directory {
    pub fn open(path: CStr) -> Result<Self, Error> {
        let fd = syscalls::open_dir(path)?;

        let mut dirents = vec![0u8; 4096];
        let mut bytes_read = syscalls::getdents64(fd, &mut dirents[..])?;
        let mut bytes_used = bytes_read;

        while bytes_read > 0 {
            if dirents.len() - bytes_used < core::mem::size_of::<libc::dirent64>() {
                dirents.reserve(4096);
                dirents.extend(core::iter::repeat(0).take(4096));
            }

            bytes_read = syscalls::getdents64(fd, &mut dirents[bytes_used..])?;
            bytes_used += bytes_read;
        }

        Ok(Self {
            fd: fd as i32,
            dirents,
            bytes_used: bytes_used as isize,
        })
    }

    pub fn iter(&'a self) -> IterDir<'a> {
        IterDir {
            directory: self,
            offset: 0,
        }
    }
}

pub struct IterDir<'a> {
    directory: &'a Directory,
    offset: isize,
}

impl<'a> Iterator for IterDir<'a> {
    type Item = RawDirEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            let dirent_ptr =
                self.directory.dirents.as_ptr().offset(self.offset) as *const libc::dirent64;

            let entry = if self.offset < self.directory.bytes_used {
                Some(RawDirEntry {
                    directory: self.directory,
                    offset: self.offset,
                    name_len: libc::strlen((*dirent_ptr).d_name.as_ptr()),
                })
            } else {
                None
            };

            self.offset += (*dirent_ptr).d_reclen as isize;

            entry
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (
            self.directory.bytes_used as usize / core::mem::size_of::<libc::dirent64>(),
            Some(
                self.directory.bytes_used as usize / (core::mem::size_of::<libc::dirent64>() - 256),
            ),
        )
    }
}

#[derive(Clone)]
pub struct RawDirEntry<'a> {
    directory: &'a Directory,
    offset: isize,
    name_len: usize,
}

impl<'a> RawDirEntry<'a> {
    fn name_ptr(&self) -> *const libc::c_char {
        unsafe {
            let dirent_ptr =
                self.directory.dirents.as_ptr().offset(self.offset) as *const libc::dirent64;
            (*dirent_ptr).d_name.as_ptr()
        }
    }

    fn _d_type(&self) -> u8 {
        unsafe {
            (*(self.directory.dirents.as_ptr().offset(self.offset) as *const libc::dirent64)).d_type
        }
    }

    pub fn name(&self) -> CStr {
        let slice =
            unsafe { core::slice::from_raw_parts(self.name_ptr() as *const u8, self.name_len + 1) };
        CStr::from_bytes(slice)
    }

    pub fn d_type(&self) -> Result<u8, Error> {
        let getdents_type = self._d_type();
        if getdents_type != libc::DT_UNKNOWN {
            Ok(getdents_type)
        } else {
            syscalls::lstatat(self.directory.fd, self.name())
                .map(|stats| (stats.st_mode & libc::S_IFMT) as u8)
        }
    }
}
