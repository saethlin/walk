mod output;

use crossbeam_deque::{Injector, Stealer, Worker};
use veneer::{CStr, Directory};

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

fn find_task<T>(local: &Worker<T>, global: &Injector<T>, stealers: &[Stealer<T>]) -> Option<T> {
    // Pop a task from the local queue, if not empty.
    local.pop().or_else(|| {
        // Otherwise, we need to look for a task elsewhere.
        std::iter::repeat_with(|| {
            // Try stealing a batch of tasks from the global queue.
            global
                .steal_batch_and_pop(local)
                // Or try stealing a task from one of the other threads.
                .or_else(|| stealers.iter().map(|s| s.steal()).collect())
        })
        // Loop while no task was stolen and any steal operation needs to be retried.
        .find(|s| !s.is_retry())
        // Extract the stolen task, if there is one.
        .and_then(|s| s.success())
    })
}

fn main() {
    let mut threads = Vec::new();
    let num_threads = num_cpus::get();
    let num_waiting = Arc::new(AtomicUsize::new(0));

    let outer_re = Arc::new(
        std::env::args()
            .nth(1)
            .map(|s| regex::Regex::new(&s).unwrap()),
    );

    let start = std::env::args()
        .nth(2)
        .map(|b| {
            b.as_bytes()
                .iter()
                .cloned()
                .chain(std::iter::once(0))
                .collect()
        })
        .unwrap_or_else(|| b".\0".to_vec());

    let injector = Arc::new(Injector::new());
    injector.push(start);

    let workers = (0..num_threads)
        .map(|_| Worker::new_fifo())
        .collect::<Vec<_>>();
    let stealers = workers.iter().map(Worker::stealer).collect::<Vec<_>>();

    for worker in workers {
        let injector = injector.clone();
        let stealers = stealers.clone();

        let num_waiting = num_waiting.clone();
        let mut path_pool = Vec::new();
        let mut buf = Vec::with_capacity(4096);
        let re = Arc::clone(&outer_re);
        threads.push(std::thread::spawn(move || {
            let mut is_waiting = false;
            'outer: loop {
                let mut current_dir_path: Vec<u8> = 'inner: loop {
                    if let Some(path) = find_task(&worker, &injector, &stealers) {
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
                let dir = if let Ok(d) = Directory::open(CStr::from_bytes(&current_dir_path)) {
                    d
                } else {
                    continue; // Ignore errors in opening directories
                };
                if current_dir_path.last() == Some(&0) {
                    current_dir_path.pop();
                }
                if current_dir_path.last() != Some(&b'/') {
                    current_dir_path.push(b'/');
                }
                let contents = if let Ok(c) = dir.read() {
                    c
                } else {
                    continue;
                };
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
                        injector.push(new_dir_path);
                    }
                    if let Ok(name_str) = std::str::from_utf8(entry.name().as_bytes()) {
                        if re
                            .as_ref()
                            .as_ref()
                            .map(|r| r.is_match(name_str))
                            .unwrap_or(true)
                        {
                            let entry_name = entry.name();
                            if (buf.len()
                                + current_dir_path.len()
                                + entry_name.as_bytes().len()
                                + 1)
                                >= buf.capacity()
                            {
                                crate::output::write_to_stdout(&buf).unwrap();
                                buf.clear();
                            }
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
}
