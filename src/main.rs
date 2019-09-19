mod cstr;
mod directory;
mod error;
mod output;
mod syscalls;

use cstr::CStr;
use directory::Directory;
use error::Error;
use output::BufferedStdout;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

fn main() -> Result<(), Error> {
    let (output_send, output_recv): (crossbeam_channel::Sender<(Vec<u8>, Directory)>, _) =
        crossbeam_channel::unbounded();

    let output_thread = std::thread::spawn(move || {
        let mut stdout = BufferedStdout::new();
        for (dir_path, dir) in output_recv.iter() {
            for entry in dir.iter() {
                if entry.name().get(0) == Some(&b'.') {
                    continue;
                }
                stdout.write(&dir_path).unwrap();
                if dir_path.last() != Some(&b'/') {
                    stdout.push(b'/').unwrap();
                }
                stdout.write(entry.name().as_bytes()).unwrap();
                stdout.push(b'\n').unwrap();
            }
        }
    });

    let mut threads = Vec::new();
    let num_threads = num_cpus::get() * 2;
    let num_waiting = Arc::new(AtomicUsize::new(0));

    let (recursion_send, recursion_recv): (
        crossbeam_channel::Sender<Vec<u8>>,
        crossbeam_channel::Receiver<Vec<u8>>,
    ) = crossbeam_channel::unbounded();

    recursion_send.send(b".\0".to_vec()).unwrap();

    for _ in 0..num_threads {
        let recv = recursion_recv.clone();
        let send = recursion_send.clone();
        let out_send = output_send.clone();
        let num_waiting = num_waiting.clone();
        threads.push(std::thread::spawn(move || {
            let mut is_waiting = false;
            'outer: loop {
                let current_dir_path = 'inner: loop {
                    if let Ok(path) = recv.recv_timeout(Duration::from_millis(10)) {
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
                // Try to avoid running out of file handles
                while out_send.len() > 1000 {
                    eprintln!("sleeping");
                    std::thread::sleep(Duration::from_millis(10));
                }
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
                for entry in dir.iter() {
                    if entry.name().as_bytes().get(0) == Some(&b'.') {
                        continue;
                    }
                    if entry.d_type().unwrap_or(libc::DT_UNKNOWN) == libc::DT_DIR {
                        let mut new_dir_path = current_dir_path.to_vec();
                        new_dir_path.pop();
                        if new_dir_path.last() != Some(&b'/') {
                            new_dir_path.push(b'/');
                        }
                        new_dir_path.extend(entry.name().as_bytes());
                        new_dir_path.push(0);
                        send.send(new_dir_path).unwrap();
                    }
                }
                out_send.send((current_dir_path, dir)).unwrap();
            }
        }));
    }
    drop(output_send);

    for t in threads {
        t.join().unwrap();
    }
    // Completion of all the walker threads will cancel the output thread
    output_thread.join().unwrap();
    Ok(())
}

/*
fn par_walk(
    path: &[u8],
    sender: crossbeam_channel::Sender<(Vec<u8>, Directory)>,
) -> Result<(), Error> {
    let dir = match Directory::open(CStr::from_bytes(path)) {
        Ok(d) => d,
        Err(_) => {
            return Ok(());
        }
    };
    let mut dir_path = Vec::new();
    for entry in dir.iter() {
        if entry.name().as_bytes().get(0) == Some(&b'.') {
            continue;
        }
        if entry.d_type()? == libc::DT_DIR {
            dir_path.clear();
            dir_path.extend_from_slice(path);
            dir_path.pop();
            if dir_path.last() != Some(&b'/') {
                dir_path.push(b'/');
            }
            dir_path.extend(entry.name().as_bytes());
            dir_path.push(0);
            par_walk(&dir_path, sender.clone())?;
        }
    }
    dir_path.clear();
    dir_path.extend_from_slice(path);
    dir_path.pop();
    sender.send((dir_path, dir)).unwrap();
    Ok(())
}
*/
