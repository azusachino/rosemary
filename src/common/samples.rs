use std::cell::Cell;
use std::rc::{Rc, Weak};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

struct ExpensiveCalculator {
    input: i32,
    cached_result: Cell<Option<i32>>, // Cache doesn't change "logical" state
}

impl ExpensiveCalculator {
    fn new(input: i32) -> Self {
        Self {
            input,
            cached_result: Cell::new(None),
        }
    }

    // ✅ Can take &self even though we mutate the cache
    fn calculate(&self) -> i32 {
        if let Some(cached) = self.cached_result.get() {
            return cached;
        }

        // Expensive computation
        let result = self.input * self.input * self.input;
        self.cached_result.set(Some(result));
        result
    }
}

struct SharedCounter {
    count: Cell<i32>,
}

trait Observer {
    fn notify(&self, event: &str); // &self, not &mut self
}

struct EventCounter {
    count: Cell<usize>,
}

impl Observer for EventCounter {
    fn notify(&self, _event: &str) {
        // ✅ Can mutate even though we only have &self
        self.count.set(self.count.get() + 1);
    }
}

fn send_event(observer: &dyn Observer) {
    observer.notify("something happened");
}

pub struct Connection {
    url: String,
    // These are internal state, not part of logical "value"
    last_ping: Cell<u64>,
    total_requests: Cell<usize>,
}

impl Connection {
    pub fn new(url: String) -> Self {
        Self {
            url,
            last_ping: Cell::new(0),
            total_requests: Cell::new(0),
        }
    }

    // ✅ Public API uses &self - cleaner interface
    pub fn send_request(&self, data: &[u8]) {
        let start = SystemTime::now();
        let duration = start.duration_since(UNIX_EPOCH).expect("what ?");
        // Update internal metrics without requiring &mut
        self.last_ping.set(duration.as_secs());
        self.total_requests.set(self.total_requests.get() + 1);

        // actual network code...
    }

    pub fn stats(&self) -> (u64, usize) {
        (self.last_ping.get(), self.total_requests.get())
    }
}

struct Node {
    value: i32,
    visit_count: Cell<usize>,         // Track visits without &mut
    parent: Cell<Option<Weak<Node>>>, // Can update parent
}

impl Node {
    fn visit(&self) {
        // ✅ Can increment counter with just &self
        self.visit_count.set(self.visit_count.get() + 1);
    }

    fn set_parent(&self, parent: Weak<Node>) {
        // ✅ Can update parent with just &self
        self.parent.set(Some(parent));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let calc = ExpensiveCalculator::new(5);
        let r1 = &calc;
        let r2 = &calc;

        println!("{}", r1.calculate());
        println!("{}", r2.calculate());
    }

    #[test]
    fn test_2() {
        let counter = Rc::new(SharedCounter {
            count: Cell::new(0),
        });

        // Multiple owners!
        let counter1 = Rc::clone(&counter);
        let counter2 = Rc::clone(&counter);
        let counter3 = Rc::clone(&counter);

        // All can mutate through shared references
        counter1.count.set(counter1.count.get() + 1);
        counter2.count.set(counter2.count.get() + 1);
        counter3.count.set(counter3.count.get() + 1);

        println!("Final count: {}", counter.count.get()); // 3
    }

    #[test]
    fn test_3() {
        let counter = EventCounter {
            count: Cell::new(0),
        };

        send_event(&counter);
        send_event(&counter);
        send_event(&counter);

        println!("Events received: {}", counter.count.get()); // 3
    }

    #[test]
    fn test_4() {
        let conn = Connection::new("https://api.example.com".into());

        // Much cleaner API - no mut needed
        conn.send_request(b"GET /data");
        conn.send_request(b"POST /update");

        println!("Stats: {:?}", conn.stats());
    }

    use std::sync::Mutex;

    #[test]
    fn test_5() {
        let n = Mutex::new(0);
        std::thread::scope(|s| {
            for _ in 0..10 {
                s.spawn(|| {
                    let mut guard = n.lock().expect("no way");
                    for _ in 0..100 {
                        *guard += 1;
                    }
                });
            }
        });
        assert_eq!(n.into_inner().expect("no way"), 1000);
    }

    use std::collections::VecDeque;
    use std::time::Duration;

    #[test]
    fn test_6() {
        let queue = Mutex::new(VecDeque::new());

        thread::scope(|s| {
            // Consuming thread
            let t = s.spawn(|| {
                loop {
                    let item = queue.lock().unwrap().pop_front();
                    if let Some(item) = item {
                        dbg!(item);
                    } else {
                        thread::park();
                    }
                }
            });

            // Producing thread
            for i in 0.. {
                queue.lock().unwrap().push_back(i);
                t.thread().unpark();
                thread::sleep(Duration::from_secs(1));
            }
        });
    }
}
