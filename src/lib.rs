//! This crate provides a data structure for storing trees ([`PackedTree`]) or forests ([`PackedForest`]) in a single [`Vec`]
//! with an overhead of only 1 `usize` per node, for fast iteration and efficient storage.
//!
//! The downsides are that you essentially have to create the entire tree in one go, and you can't modify its structure
//! after it's created. You *can* mutably iterate over the tree though (see [`PackedTree::root_mut`] or [`PackedForest::iter_trees_mut`])
//! and you can modify the values of the nodes that way, but not the structure of the tree.
//!
//! Also, a node doesn't know how many children it has
//! without iterating over all of them. If you need to know that, see [`ExactSizePackedTree`] and [`ExactSizePackedForest`],
//! which do keep track of the number of children each node has (but they store 1 extra `usize` per node).
//!
//! # Example
//! ```
//! use packed_tree::{PackedTree, NodeRef};
//!
//! // Create the tree
//! let tree = PackedTree::new("the root node", |node_builder| {
//! 	node_builder.add_child("a node without children");
//! 	node_builder.build_child("a node with children", |node_builder| {
//! 		node_builder.add_child("another node without children");
//! 	});
//! });
//!
//! // Iterate it, printing the values and counting the number of nodes
//! fn count_num_nodes(node: NodeRef<&'static str>) -> usize {
//! 	println!("Counting node \"{}\"", node.val());
//! 	let mut result = 1;
//! 	for child in node.children() {
//! 		result += count_num_nodes(child);
//! 	}
//! 	result
//! }
//!
//! assert_eq!(count_num_nodes(tree.root()), 4);
//! ```

#[macro_use]
extern crate derive_destructure;

mod core;
mod tree;
mod exactsize;
mod serde;
mod test;
mod extra;

pub use crate::core::*;
pub use crate::exactsize::*;
pub use crate::tree::*;
