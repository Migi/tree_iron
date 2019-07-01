#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::black_box;

use tree_iron::{IronedTree, NodeBuilder};

use rand::{Rng, SeedableRng};
use rand::distributions::{Distribution, Uniform};

use failure::Fallible;

trait NodeCreator : Sized {
    type ValType;
    fn val(&self) -> Self::ValType;
    fn next_child(&mut self, rng: &mut impl Rng) -> Option<Self>;
}

// Creates a tree where at each level, every node has the same number of children
struct SimpleNodeCreator<'a> {
    val: u32,
    depth: usize,
    num_children_created: usize,
    children_per_node_per_level: &'a [usize],
}

impl<'a> NodeCreator for SimpleNodeCreator<'a> {
    type ValType = u32;

    fn val(&self) -> Self::ValType {
        self.val
    }

    fn next_child(&mut self, rng: &mut impl Rng) -> Option<Self> {
        self.children_per_node_per_level.get(self.depth).and_then(|max_children| {
            if self.num_children_created < *max_children {
                self.num_children_created += 1;
                Some(SimpleNodeCreator {
                    val: rng.next_u32(),
                    depth: self.depth+1,
                    num_children_created: 0,
                    children_per_node_per_level: self.children_per_node_per_level,
                })
            } else {
                None
            }
        })
    }
}

fn make_wide_flat_tree() -> impl NodeCreator {
    SimpleNodeCreator {
        val: 1,
        depth: 0,
        num_children_created: 0,
        children_per_node_per_level: &[1000]
    }
}

fn make_binary_tree() -> impl NodeCreator {
    SimpleNodeCreator {
        val: 1,
        depth: 0,
        num_children_created: 0,
        children_per_node_per_level: &[2, 2, 2, 2, 2, 2, 2, 2, 2, 2]
    }
}


// Creates a tree where at each level, every node has the same number of children
struct RandomNodeCreator<'a> {
    val: u32,
    depth: usize,
    child_chance_per_level: &'a [f64],
}

impl<'a> NodeCreator for RandomNodeCreator<'a> {
    type ValType = u32;

    fn val(&self) -> Self::ValType {
        self.val
    }

    fn next_child(&mut self, rng: &mut impl Rng) -> Option<Self> {
        self.child_chance_per_level.get(self.depth).and_then(|child_chance| {
            let range = Uniform::new(0.0f64, 1.0);
            if range.sample(rng) < *child_chance {
                Some(RandomNodeCreator {
                    val: rng.next_u32(),
                    depth: self.depth+1,
                    child_chance_per_level: self.child_chance_per_level,
                })
            } else {
                None
            }
        })
    }
}

fn make_wide_random_tree() -> impl NodeCreator {
    RandomNodeCreator {
        val: 1,
        depth: 0,
        child_chance_per_level: &[0.99, 0.9]
    }
}

fn make_deep_random_tree() -> impl NodeCreator {
    RandomNodeCreator {
        val: 1,
        depth: 0,
        child_chance_per_level: &[2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3.]
    }
}

fn create_ironed_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, ironed_node_creator: &mut NodeBuilder<C::ValType>) {
    while let Some(mut child_creator) = creator.next_child(rng) {
        ironed_node_creator.build_child(child_creator.val(), |child_ironed_node_creator| {
            create_ironed_tree_rec(&mut child_creator, rng, child_ironed_node_creator);
        });
    }
}

fn create_ironed_tree<C: NodeCreator>(mut creator: C, rng: &mut impl Rng) -> IronedTree<C::ValType> {
    IronedTree::new(creator.val(), |ironed_node_creator| {
        create_ironed_tree_rec(&mut creator, rng, ironed_node_creator);
    })
}

fn create_index_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, node: indextree::NodeId, arena: &mut indextree::Arena<C::ValType>) -> Fallible<()> {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let child_node = arena.new_node(child_creator.val());
        node.append(child_node, arena)?;
        create_index_tree_rec(&mut child_creator, rng, child_node, arena)?;
    }
    Ok(())
}

fn create_index_tree<C: NodeCreator>(mut creator: C, rng: &mut impl Rng, arena: &mut indextree::Arena<C::ValType>) -> Fallible<indextree::NodeId> {
    let root = arena.new_node(creator.val());
    create_index_tree_rec(&mut creator, rng, root, arena)?;
    Ok(root)
}

fn create_id_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, node: id_tree::NodeId, tree: &mut id_tree::Tree<C::ValType>) -> Result<(), id_tree::NodeIdError> {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let child_id = tree.insert(id_tree::Node::new(child_creator.val()), id_tree::InsertBehavior::UnderNode(&node))?;
        create_id_tree_rec(&mut child_creator, rng, child_id, tree)?;
    }
    Ok(())
}

fn create_id_tree<C: NodeCreator>(mut creator: C, rng: &mut impl Rng) -> Result<id_tree::Tree<C::ValType>, id_tree::NodeIdError> {
    let mut tree = id_tree::TreeBuilder::new()
        .build();
    let root_id = tree.insert(id_tree::Node::new(creator.val()), id_tree::InsertBehavior::AsRoot)?;
    create_id_tree_rec(&mut creator, rng, root_id, &mut tree)?;
    Ok(tree)
}

fn create_ego_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, mut node: ego_tree::NodeMut<C::ValType>) {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let child_node = node.append(child_creator.val());
        create_ego_tree_rec(&mut child_creator, rng, child_node);
    }
}

fn create_ego_tree<C: NodeCreator>(mut creator: C, rng: &mut impl Rng) -> ego_tree::Tree<C::ValType> {
    let mut tree = ego_tree::Tree::new(creator.val());
    let root = tree.root_mut();
    create_ego_tree_rec(&mut creator, rng, root);
    tree
}

fn create_vec_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, node_id: vec_tree::Index, tree: &mut vec_tree::VecTree<C::ValType>) {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let child_node_id = tree.insert(child_creator.val(), node_id);
        create_vec_tree_rec(&mut child_creator, rng, child_node_id, tree);
    }
}

fn create_vec_tree<C: NodeCreator>(mut creator: C, rng: &mut impl Rng) -> vec_tree::VecTree<C::ValType> {
    let mut tree = vec_tree::VecTree::new();
    let root_id = tree.insert_root(creator.val());
    create_vec_tree_rec(&mut creator, rng, root_id, &mut tree);
    tree
}

pub struct NaiveNode<T> {
    pub value: T,
    children: Vec<T>
}

fn create_naive_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, node: &mut NaiveNode<C::ValType>) {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let mut child_node = NaiveNode {
            value: child_creator.val(),
            children: vec![]
        };
        create_naive_tree_rec(&mut child_creator, rng, &mut child_node);
        node.children.push(child_creator.val());
    }
}

fn create_naive_tree<C: NodeCreator>(mut creator: C, rng: &mut impl Rng) -> NaiveNode<C::ValType> {
    let mut root = NaiveNode {
        value: creator.val(),
        children: vec![]
    };
    create_naive_tree_rec(&mut creator, rng, &mut root);
    root
}

fn make_rng() -> impl Rng {
    rand_xorshift::XorShiftRng::seed_from_u64(1234)
}

fn benchmark_tree_type<C: NodeCreator + 'static>(c: &mut Criterion, creator: fn() -> C, type_name: &'static str) {
    c.bench_function(&format!("{}_ironed", type_name), move |b| {
        b.iter(|| {
            create_ironed_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("{}_indextree", type_name), move |b| {
        b.iter(|| {
            let mut arena = indextree::Arena::new();
            create_index_tree(creator(), &mut black_box(make_rng()), &mut arena).unwrap()
        });
    });
    c.bench_function(&format!("{}_id_tree", type_name), move |b| {
        b.iter(|| {
            create_id_tree(creator(), &mut black_box(make_rng())).unwrap()
        });
    });
    c.bench_function(&format!("{}_ego_tree", type_name), move |b| {
        b.iter(|| {
            create_ego_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("{}_vec_tree", type_name), move |b| {
        b.iter(|| {
            create_vec_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("{}_naive_tree", type_name), move |b| {
        b.iter(|| {
            create_naive_tree(creator(), &mut black_box(make_rng()))
        });
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    benchmark_tree_type(c, make_wide_flat_tree, "flat_wide");
    benchmark_tree_type(c, make_binary_tree, "binary");
    benchmark_tree_type(c, make_wide_random_tree, "wide_random");
    benchmark_tree_type(c, make_deep_random_tree, "deep_random");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
