//! This crate provides a data structure for storing trees ([`IronedTree`]) or forests ([`IronedForest`]) in a single [`Vec`]
//! with an overhead of only 1 `usize` per node, for fast iteration and efficient storage.
//!
//! The downsides are that you essentially have to create the entire tree in one go, and you can't modify its structure
//! after it's created. You *can* mutably iterate over the tree though (see [`IronedTree::root_mut`] or [`IronedForest::iter_trees_mut`])
//! and you can modify the values of the nodes that way, but not the structure of the tree.
//!
//! Also, a node doesn't know how many children it has
//! without iterating over all of them. If you need to know that, see [`ExactSizeIronedTree`] and [`ExactSizeIronedForest`],
//! which do keep track of the number of children each node has (but of course it stores 1 extra `usize` per node).
//!
//! # Example
//! ```
//! use tree_iron::{IronedTree, NodeRef};
//!
//! // Create the tree
//! let tree = IronedTree::new("the root node", |mut node| {
//! 	node.add_child("a node without children");
//! 	node.build_child("a node with children", |mut node| {
//! 		node.add_child("another node without children");
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

mod core;
mod tree;
mod exactsize;
mod serde;
mod test;

pub use crate::core::*;
pub use crate::exactsize::*;
pub use crate::tree::*;
