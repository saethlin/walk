mod output;

use veneer::{CStr, Directory, Error};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn main() -> Result<(), Error> {
    let mut threads = Vec::new();
    let num_threads = num_cpus::get();
    let num_waiting = Arc::new(AtomicUsize::new(0));

    let (recursion_send, recursion_recv): (crossbeam_channel::Sender<Vec<u8>>, _) =
        crossbeam_channel::unbounded();

    recursion_send.send(b"/\0".to_vec()).unwrap();

    for _ in 0..num_threads {
        let recv = recursion_recv.clone();
        let send = recursion_send.clone();
        let num_waiting = num_waiting.clone();
        let mut path_pool = Vec::new();
        let mut buf = Vec::with_capacity(4096);
        threads.push(std::thread::spawn(move || {
            let mut is_waiting = false;
            'outer: loop {
                let mut current_dir_path = 'inner: loop {
                    if let Ok(path) = recv.try_recv() {
                        if is_waiting {
                            // Exit the waiting state if we were in it before
                            num_waiting.fetch_sub(1, Ordering::SeqCst);
                            is_waiting = false;
                        }
                        break 'inner path;
                    } else {
                        if !is_waiting {
                            num_waiting.fetch_add(1, Ordering::SeqCst);
                            is_waiting = true;
                        } else if num_waiting.load(Ordering::SeqCst) == num_threads {
                            break 'outer;
                        }
                    }
                };
                let dir = match Directory::open(CStr::from_bytes(&current_dir_path)) {
                    Ok(d) => d,
                    Err(e) => {
                        if e.0 != 13 {
                            eprintln!(
                                "{:?} {:?}",
                                e,
                                std::str::from_utf8(
                                    &current_dir_path[..current_dir_path.len() - 1]
                                )
                                .unwrap_or("??")
                            );
                        }
                        continue; // Ignore errors in opening directories
                    }
                };
                if current_dir_path.last() == Some(&0) {
                    current_dir_path.pop();
                }
                if current_dir_path.last() != Some(&b'/') {
                    current_dir_path.push(b'/');
                }
                let contents = dir.read().unwrap();
                for entry in contents.iter() {
                    /*
                    if entry.name().as_bytes().get(0) == Some(&b'.') {
                        continue;
                    }
                    */
                    if entry.name().as_bytes() == b"." || entry.name().as_bytes() == b".." {
                        continue;
                    }
                    if entry.d_type() == veneer::directory::DType::DIR {
                        let mut new_dir_path =
                            path_pool.pop().unwrap_or_else(|| Vec::with_capacity(256));
                        new_dir_path.clear();
                        new_dir_path
                            .reserve(current_dir_path.len() + entry.name().as_bytes().len() + 1);
                        new_dir_path.extend(&current_dir_path);
                        new_dir_path.extend(entry.name().as_bytes());
                        new_dir_path.push(0);
                        send.send(new_dir_path).unwrap();
                    } else {
                        let entry_name = entry.name();
                        if (buf.len() + current_dir_path.len() + entry_name.as_bytes().len() + 1)
                            < buf.capacity()
                        {
                            buf.extend(&current_dir_path);
                            buf.extend(entry_name.as_bytes());
                            buf.push(b'\n');
                        } else {
                            crate::output::write_to_stdout(&buf).unwrap();
                            buf.clear();
                            buf.extend(&current_dir_path);
                            buf.extend(entry_name.as_bytes());
                            buf.push(b'\n');
                        }
                    }
                }
                path_pool.push(current_dir_path);
            }
            crate::output::write_to_stdout(&buf).unwrap();
        }));
    }

    for t in threads {
        t.join().unwrap();
    }

    Ok(())
}
