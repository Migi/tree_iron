#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::black_box;

use rand::Rng;

trait NodeCreator<'a> {
    type ValType;
    fn val(&self) -> Self::ValType;
    fn next_child<'b>(&'b mut self) -> Option<impl NodeCreator<'b>>;
}

// Creates a tree where at each level, every node has the same number of children
struct SimpleNodeCreator<'a, 'r, R: Rng> {
    depth: usize,
    num_children_created: usize,
    children_per_node_per_level: &'a [usize],
    rng: &'r mut R,
}

impl<'a, 'r, R: Rng> NodeCreator<'r> for SimpleNodeCreator<'a,'r,R> {
    type ValType = i32;

    fn val(&self) -> Self::ValType {
        7
    }

    fn next_child(&mut self) -> Option<SimpleNodeCreator<R>> {
        self.children_per_node_per_level.get(self.depth).map(|max_children| {
            if self.num_children_created < max_children {
                self.num_children_created += 1;
                Some(SimpleNodeCreator {
                    depth: self.depth+1,
                    num_children_created: 0,
                    children_per_node_per_level: self.children_per_node_per_level,
                    rng: &mut self.rng
                })
            } else {
                None
            }
        })
    }
}

static SINGLE_LEVEL_1000_NODE_TREE: &'static [usize] = &[1000];

fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("fib 20", |b| b.iter(|| fibonacci(black_box(20))));
    c.bench_function("fib 21", |b| b.iter(|| fibonacci(black_box(21))));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
