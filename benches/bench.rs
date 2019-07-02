#[macro_use]
extern crate criterion;

use criterion::Criterion;
use criterion::black_box;

use tree_iron::{IronedTree, NodeBuilder};

use rand::{Rng, SeedableRng};
use rand::distributions::{Distribution, Uniform};

use failure::Fallible;
use std::hash::{Hash,Hasher};

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

fn make_flat_tree() -> SimpleNodeCreator<'static> {
    SimpleNodeCreator {
        val: 1,
        depth: 0,
        num_children_created: 0,
        children_per_node_per_level: &[1000]
    }
}

fn make_shallow_tree() -> SimpleNodeCreator<'static> {
    SimpleNodeCreator {
        val: 1,
        depth: 0,
        num_children_created: 0,
        children_per_node_per_level: &[100,10]
    }
}

fn make_binary_tree() -> SimpleNodeCreator<'static> {
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

fn make_wide_random_tree() -> RandomNodeCreator<'static> {
    RandomNodeCreator {
        val: 1,
        depth: 0,
        child_chance_per_level: &[0.99, 0.9]
    }
}

fn make_deep_random_tree() -> RandomNodeCreator<'static> {
    RandomNodeCreator {
        val: 1,
        depth: 0,
        child_chance_per_level: &[2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3., 2./3.]
    }
}

trait TreeVisitor<T> {
    fn visit_node(&mut self, node: impl VisitableNode<T>);
}

trait VisitableNode<T> {
    fn val(&self) -> &T;
    fn visit_children(self, v: impl TreeVisitor<T>);
}

struct NodesPerLevelCounter<'a> {
    nodes_per_level: &'a mut Vec<usize>,
    cur_level: usize,
}

impl<'a,T> TreeVisitor<T> for NodesPerLevelCounter<'a> {
    fn visit_node(&mut self, node: impl VisitableNode<T>) {
        if let Some(nodes) = self.nodes_per_level.get_mut(self.cur_level) {
            *nodes += 1;
        } else {
            self.nodes_per_level.push(1);
        }
        node.visit_children(NodesPerLevelCounter {
            nodes_per_level: &mut self.nodes_per_level,
            cur_level: self.cur_level+1
        });
    }
}

fn count_nodes_per_level<T>(root: impl VisitableNode<T>) -> Vec<usize> {
    let mut result = vec![1];
    root.visit_children(NodesPerLevelCounter {
        nodes_per_level: &mut result,
        cur_level: 1,
    });
    result
}


struct TreeHasher<'a> {
    hasher: &'a mut twox_hash::XxHash64,
}

impl<'a,T:Hash> TreeVisitor<T> for TreeHasher<'a> {
    fn visit_node(&mut self, node: impl VisitableNode<T>) {
        node.val().hash(self.hasher);
        let mut hasher = twox_hash::XxHash64::with_seed(123456789);
        node.visit_children(TreeHasher {
            hasher: &mut hasher,
        });
        self.hasher.write_u64(hasher.finish());
    }
}

fn hash_tree<T:Hash>(root: impl VisitableNode<T>) -> u64 {
    let mut hasher = twox_hash::XxHash64::with_seed(123456789);
    root.val().hash(&mut hasher);
    root.visit_children(TreeHasher {
        hasher: &mut hasher,
    });
    hasher.finish()
}

// ================ Here begin the implementations of the libraries

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

impl<'a,T> VisitableNode<T> for tree_iron::NodeRef<'a,T> {
    fn val(&self) -> &T {
        self.val()
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.children() {
            v.visit_node(child);
        }
    }
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

struct IndexTreeNode<'a,T> {
    id: indextree::NodeId,
    arena: &'a indextree::Arena<T>
}

impl<'a,T> VisitableNode<T> for IndexTreeNode<'a,T> {
    fn val(&self) -> &T {
        &self.arena[self.id].data
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.id.children(self.arena) {
            v.visit_node(IndexTreeNode {
                id: child,
                arena: self.arena
            });
        }
    }
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

struct IdTreeNode<'a,T> {
    node: &'a id_tree::Node<T>,
    tree: &'a id_tree::Tree<T>
}

impl<'a,T> VisitableNode<T> for IdTreeNode<'a,T> {
    fn val(&self) -> &T {
        &self.node.data()
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.node.children() {
            v.visit_node(IdTreeNode {
                node: self.tree.get(child).unwrap(),
                tree: self.tree
            });
        }
    }
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

impl<'a,T> VisitableNode<T> for ego_tree::NodeRef<'a,T> {
    fn val(&self) -> &T {
        self.value()
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.children() {
            v.visit_node(child);
        }
    }
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

struct VecTreeNode<'a,T> {
    id: vec_tree::Index,
    tree: &'a vec_tree::VecTree<T>
}

impl<'a,T> VisitableNode<T> for VecTreeNode<'a,T> {
    fn val(&self) -> &T {
        &self.tree.get(self.id).unwrap()
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.tree.children(self.id) {
            v.visit_node(VecTreeNode {
                id: child,
                tree: self.tree
            });
        }
    }
}

pub struct NaiveNode<T> {
    pub value: T,
    children: Vec<NaiveNode<T>>
}

fn create_naive_tree_rec<C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, node: &mut NaiveNode<C::ValType>) {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let mut child_node = NaiveNode {
            value: child_creator.val(),
            children: vec![]
        };
        create_naive_tree_rec(&mut child_creator, rng, &mut child_node);
        node.children.push(child_node);
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

impl<'a,T> VisitableNode<T> for &'a NaiveNode<T> {
    fn val(&self) -> &T {
        &self.value
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.children.iter() {
            v.visit_node(child);
        }
    }
}

pub struct BumpNode<'bump,T> {
    pub value: T,
    children: bumpalo::collections::Vec<'bump, BumpNode<'bump, T>>
}

fn create_bump_tree_rec<'bump, C: NodeCreator>(creator: &mut C, rng: &mut impl Rng, node: &mut BumpNode<'bump, C::ValType>, bump: &'bump bumpalo::Bump) {
    while let Some(mut child_creator) = creator.next_child(rng) {
        let mut child_node = BumpNode {
            value: child_creator.val(),
            children: bumpalo::collections::Vec::new_in(bump)
        };
        create_bump_tree_rec(&mut child_creator, rng, &mut child_node, bump);
        node.children.push(child_node);
    }
}

fn create_bump_tree<'bump, C: NodeCreator>(mut creator: C, rng: &mut impl Rng, bump: &'bump bumpalo::Bump) -> BumpNode<'bump, C::ValType> {
    let mut root = BumpNode {
        value: creator.val(),
        children: bumpalo::collections::Vec::new_in(bump)
    };
    create_bump_tree_rec(&mut creator, rng, &mut root, bump);
    root
}

impl<'a,'bump,T> VisitableNode<T> for &'a BumpNode<'bump,T> {
    fn val(&self) -> &T {
        &self.value
    }
    fn visit_children(self, mut v: impl TreeVisitor<T>) {
        for child in self.children.iter() {
            v.visit_node(child);
        }
    }
}

fn make_rng() -> impl Rng {
    rand_xorshift::XorShiftRng::seed_from_u64(123456789)
}

fn benchmark_tree_type<C: NodeCreator + 'static>(c: &mut Criterion, creator: fn() -> C, type_name: &'static str) where C::ValType: Hash {
    c.bench_function(&format!("make_{}_ironed", type_name), move |b| {
        b.iter(|| {
            create_ironed_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("hash_{}_ironed", type_name), move |b| {
        let tree = create_ironed_tree(creator(), &mut black_box(make_rng()));
        b.iter(|| {
            hash_tree(tree.root())
        });
    });
    c.bench_function(&format!("make_{}_indextree", type_name), move |b| {
        b.iter(|| {
            let mut arena = indextree::Arena::new();
            create_index_tree(creator(), &mut black_box(make_rng()), &mut arena).unwrap()
        });
    });
    c.bench_function(&format!("hash_{}_indextree", type_name), move |b| {
        let mut arena = indextree::Arena::new();
        let tree = create_index_tree(creator(), &mut black_box(make_rng()), &mut arena).unwrap();
        b.iter(|| {
            hash_tree(IndexTreeNode {
                id: tree,
                arena: &arena
            })
        });
    });
    c.bench_function(&format!("make_{}_id_tree", type_name), move |b| {
        b.iter(|| {
            create_id_tree(creator(), &mut black_box(make_rng())).unwrap()
        });
    });
    c.bench_function(&format!("hash_{}_id_tree", type_name), move |b| {
        let tree = create_id_tree(creator(), &mut black_box(make_rng())).unwrap();
        b.iter(|| {
            hash_tree(IdTreeNode {
                node: tree.get(tree.root_node_id().unwrap()).unwrap(),
                tree: &tree
            })
        });
    });
    c.bench_function(&format!("make_{}_ego_tree", type_name), move |b| {
        b.iter(|| {
            create_ego_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("hash_{}_ego_tree", type_name), move |b| {
        let tree = create_ego_tree(creator(), &mut black_box(make_rng()));
        b.iter(|| {
            hash_tree(tree.root())
        });
    });
    c.bench_function(&format!("make_{}_vec_tree", type_name), move |b| {
        b.iter(|| {
            create_vec_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("hash_{}_vec_tree", type_name), move |b| {
        let tree = create_vec_tree(creator(), &mut black_box(make_rng()));
        b.iter(|| {
            hash_tree(VecTreeNode {
                id: tree.get_root_index().unwrap(),
                tree: &tree
            })
        });
    });
    c.bench_function(&format!("make_{}_naive_tree", type_name), move |b| {
        b.iter(|| {
            create_naive_tree(creator(), &mut black_box(make_rng()))
        });
    });
    c.bench_function(&format!("hash_{}_naive_tree", type_name), move |b| {
        let tree = create_naive_tree(creator(), &mut black_box(make_rng()));
        b.iter(|| {
            hash_tree(&tree)
        });
    });
    c.bench_function(&format!("make_{}_bump_tree", type_name), move |b| {
        b.iter(|| {
            let bump = bumpalo::Bump::new();
            black_box(create_bump_tree(creator(), &mut black_box(make_rng()), &bump));
        });
    });
    c.bench_function(&format!("hash_{}_bump_tree", type_name), move |b| {
        let bump = bumpalo::Bump::new();
        let tree = create_bump_tree(creator(), &mut black_box(make_rng()), &bump);
        b.iter(|| {
            hash_tree(&tree)
        });
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    benchmark_tree_type(c, make_flat_tree, "flat");
    benchmark_tree_type(c, make_shallow_tree, "shallow");
    benchmark_tree_type(c, make_binary_tree, "binary");
    benchmark_tree_type(c, make_wide_random_tree, "wide_random");
    benchmark_tree_type(c, make_deep_random_tree, "deep_random");
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
