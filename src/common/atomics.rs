use std::sync::atomic::Ordering::*;
use std::sync::atomic::*;

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use super::*;

    // stop flag
    #[test]
    fn test_stop_flag() {
        static STOP: AtomicBool = AtomicBool::new(false);
        let background = thread::spawn(|| {
            while !STOP.load(Ordering::Relaxed) {
                println!("hajimi manbo!");
                thread::sleep(Duration::from_secs(1));
            }
        });

        for line in std::io::stdin().lines() {
            match line.expect("no way").as_str() {
                "help" => println!("commands: help, stop"),
                "stop" => break,
                cmd => println!("unknown command: {cmd:?}"),
            }
        }

        STOP.store(true, Ordering::Relaxed);
        background.join().expect("no way");
        assert!(true);
    }

    #[test]
    fn test_progress_report() {
        let num_done = AtomicUsize::new(0);

        thread::scope(|s| {
            // A background thread to process all 100 items.
            s.spawn(|| {
                for i in 0..100 {
                    println!("processed {i:?}");
                    num_done.store(i + 1, Ordering::Relaxed);
                }
            });

            // The main thread shows status updates, every second.
            loop {
                let n = num_done.load(Ordering::Relaxed);
                if n == 100 {
                    break;
                }
                println!("Working.. {n}/100 done");
                thread::sleep(Duration::from_secs(1));
            }
        });

        println!("Done!");
    }

    // notify the main thread while all tasks completed
    #[test]
    fn test_synchronization() {
        let num_done = AtomicUsize::new(0);

        let main_thread = thread::current();

        thread::scope(|s| {
            // A background thread to process all 100 items.
            s.spawn(|| {
                for i in 0..100 {
                    println!("processing {i:?}");
                    num_done.store(i + 1, Ordering::Relaxed);
                    main_thread.unpark(); // Wake up the main thread.
                }
            });

            // The main thread shows status updates.
            loop {
                let n = num_done.load(Ordering::Relaxed);
                if n == 100 {
                    break;
                }
                println!("Working.. {n}/100 done");
                thread::park_timeout(Duration::from_secs(1));
            }
        });

        println!("Done!");
    }

    // lazy initialization
    fn get_x() -> u64 {
        static X: AtomicU64 = AtomicU64::new(0);
        let mut x = X.load(Relaxed);
        if x == 0 {
            x = calculate_x();
            X.store(x, Relaxed);
        }
        x
    }

    fn calculate_x() -> u64 {
        // simulate very heavy operation
        thread::sleep(Duration::from_secs(2));
        3939u64
    }

    #[test]
    fn test_fetch_add() {
        let a = AtomicI32::new(100);
        let b = a.fetch_add(23, Relaxed);
        let c = a.load(Relaxed);

        assert_eq!(b, 100);
        assert_eq!(c, 123);
    }

    fn process_item(i: i32) {
        let my_id = std::thread::current().id();
        println!("[{my_id:?}] I'm doing my part {i:?}");
    }

    #[test]
    fn test_report_progress_conc() {
        let num_done = &AtomicUsize::new(0);

        thread::scope(|s| {
            // Four background threads to process all 100 items, 25 each.
            for t in 0..4 {
                s.spawn(move || {
                    for i in 0..25 {
                        process_item(t * 25 + i); // Assuming this takes some time.
                        num_done.fetch_add(1, Relaxed);
                    }
                });
            }

            // The main thread shows status updates, every second.
            loop {
                let n = num_done.load(Relaxed);
                if n == 100 {
                    break;
                }
                println!("Working.. {n}/100 done");
                thread::sleep(Duration::from_millis(10));
            }
        });

        println!("Done!");
    }

    #[test]
    fn test_statistics() {
        let num_done = &AtomicUsize::new(0);
        let total_time = &AtomicU64::new(0);
        let max_time = &AtomicU64::new(0);

        thread::scope(|s| {
            // Four background threads to process all 100 items, 25 each.
            for t in 0..4 {
                s.spawn(move || {
                    for i in 0..25 {
                        let start = Instant::now();
                        process_item(t * 25 + i); // Assuming this takes some time.
                        let time_taken = start.elapsed().as_micros() as u64;
                        num_done.fetch_add(1, Relaxed);
                        total_time.fetch_add(time_taken, Relaxed);
                        max_time.fetch_max(time_taken, Relaxed);
                    }
                });
            }

            // The main thread shows status updates, every second.
            loop {
                let total_time = Duration::from_micros(total_time.load(Relaxed));
                let max_time = Duration::from_micros(max_time.load(Relaxed));
                let n = num_done.load(Relaxed);
                if n == 100 {
                    break;
                }
                if n == 0 {
                    println!("Working.. nothing done yet.");
                } else {
                    println!(
                        "Working.. {n}/100 done, {:?} average, {:?} peak",
                        total_time / n as u32,
                        max_time,
                    );
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        println!("Done!");
    }
}
