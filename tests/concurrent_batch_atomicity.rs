use std::sync::{Arc, Barrier};
use std::thread;

use sled::{Config, Db};

const CONCURRENCY: usize = 32;
const N_KEYS: usize = 1024;
const LEAF_FANOUT: usize = 8;

fn batch_writer(
    db: Db<64, LEAF_FANOUT, 128>,
    barrier: Arc<Barrier>,
    thread_number: usize,
) {
    barrier.wait();
    let mut batch = sled::Batch::default();
    for key_number in 0_u128..N_KEYS as _ {
        // LE is intentionally a little scrambled
        batch.insert(&key_number.to_le_bytes(), &thread_number.to_le_bytes());
    }

    db.apply_batch(batch).unwrap();
}

#[test]
fn concurrent_batch_atomicity() {
    let db: Db<64, LEAF_FANOUT, 128> = Config {
        path: "concurrent_batch_atomicity".into(),
        ..Default::default()
    }
    .open()
    .unwrap();

    let mut threads = vec![];

    let barrier = Arc::new(Barrier::new(CONCURRENCY + 1));
    for thread_number in 0..CONCURRENCY {
        let db = db.clone();
        let barrier = barrier.clone();
        let jh =
            thread::spawn(move || batch_writer(db, barrier, thread_number));
        threads.push(jh);
    }

    barrier.wait();
    let before = std::time::Instant::now();

    for thread in threads.into_iter() {
        thread.join().unwrap();
    }

    println!("writers took {:?}", before.elapsed());

    let mut expected_v = None;

    for key_number in 0_u128..N_KEYS as _ {
        let actual_v = db.get(&key_number.to_le_bytes()).unwrap().unwrap();
        if expected_v.is_none() {
            expected_v = Some(actual_v.clone());
        }
        assert_eq!(Some(actual_v), expected_v);
    }

    let _ = std::fs::remove_dir_all("concurrent_batch_atomicity");
}
