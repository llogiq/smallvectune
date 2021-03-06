extern crate smallvectune as smallvec;
extern crate rand;

use rand::Rng;
use smallvec::SmallVec;

fn main() {
    let mut x: SmallVec<[SmallVec<[u8; 2]>; 1]> = SmallVec::new();
    let mut rng = rand::thread_rng();
    for _ in 0..1000 {
        match rng.gen_range(0, 3) {
            0 => x.push(SmallVec::new()),
            1 => {
                let len = { x.len() };
                if len > 0 {
                    x[rng.gen_range(0, len)].push(0u8)
                } else {
                    x.push(SmallVec::new())
                }
            }
            2 => {
                if x.is_empty() { continue; }
                let idx = { rng.gen_range(0, x.len()) };
                if x[idx].pop().is_none() {
                    x.pop();
                }
            }
            _ => unreachable!()
        }
    }
}
