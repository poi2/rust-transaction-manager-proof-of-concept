use std::sync::{Arc, Mutex};
use std::time::Instant;

fn main() {
    let n = 1_000_000;

    let start = Instant::now();
    for _ in 0..n {
        let data = Arc::new(Mutex::new(0));
        for _ in 0..10 {
            let _lock = data.lock().unwrap();
        }
    }
    let duration = start.elapsed();

    println!("Total time: {:?}", duration);
    println!("Per Arc<Mutex> creation + 10 locks: {:?}", duration / n);
}
