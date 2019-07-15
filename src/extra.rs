// This file contains all functions and trait implementations of PackedForest and related types
// that don't require unsafe.

use crate::*;

use std::fmt::{Debug, Formatter};

impl<T> PackedForest<T> {
    /// Build a tree with the given root value, and add it to the forest.
    ///
    /// The parameter `root_val` is the value that the root node of the tree will have.
    ///
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a `&mut `[`NodeBuilder`] that can be
    /// used to add nodes to the root node. The value returned by `node_builder_cb` becomes the return value of this function.
    ///
    /// For complex use cases where callbacks can get in the way, [`get_tree_builder`](`PackedForest::get_tree_builder`) may be more ergonomic.
    #[inline]
    pub fn build_tree<R>(
        &mut self,
        root_val: T,
        node_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> R,
    ) -> R {
        let mut builder = self.get_tree_builder();
        let ret = node_builder_cb(&mut builder);
        builder.finish(root_val);
        ret
    }

    /// Build a tree, where value of the root node comes from the return value of the given closure, and add it to the forest.
    /// 
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a `&mut `[`NodeBuilder`] that can be
    /// used to add nodes to the root node. The value returned by `node_builder_cb` becomes the value of the root node of the tree.
    /// 
    /// For complex use cases where callbacks can get in the way, [`get_tree_builder`](`PackedForest::get_tree_builder`) may be more ergonomic.
    #[inline]
    pub fn build_tree_by_ret_val(
        &mut self,
        node_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> T,
    ) {
        let mut builder = self.get_tree_builder();
        let root_val = node_builder_cb(&mut builder);
        builder.finish(root_val);
    }

    /// Add a tree with only a single node to the forest. The parameter `val` is the value of that single node.
    #[inline]
    pub fn add_single_node_tree(&mut self, val: T) {
        self.get_tree_builder().finish(val);
    }
}

fn fmt_node<T: Debug>(node: NodeRef<T>, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{ value: {:?}, children: [", node.val())?;
    for child in node.children() {
        fmt_node(child, f)?;
    }
    write!(f, "]}}")
}

impl<T: Debug> Debug for PackedForest<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PackedForest [")?;
        for tree in self.iter_trees() {
            fmt_node(tree, f)?;
        }
        write!(f, "]")
    }
}

impl<T: Debug> Debug for PackedTree<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "PackedTree")?;
        fmt_node(self.root(), f)
    }
}

fn fmt_exact_size_node<T: Debug>(node: ExactSizeNodeRef<T>, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{{ value: {:?}, children: [", node.val())?;
    for child in node.children() {
        fmt_exact_size_node(child, f)?;
    }
    write!(f, "]}}")
}

impl<T: Debug> Debug for ExactSizePackedForest<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExactSizePackedForest [")?;
        for tree in self.iter_trees() {
            fmt_exact_size_node(tree, f)?;
        }
        write!(f, "]")
    }
}

impl<T: Debug> Debug for ExactSizePackedTree<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "ExactSizePackedTree")?;
        fmt_exact_size_node(self.root(), f)
    }
}

impl<'a,T> NodeBuilder<'a,T> {
    /// Build a child node with the given value, and add it to the tree as a child of the node
    /// that is being built by the current [`NodeBuilder`].
    ///
    /// The parameter `val` is the value that the child node will have.
    ///
    /// The parameter `child_builder_cb` is a callback function that is called exactly once. It is passed a `&mut `[`NodeBuilder`] that can be
    /// used to add children to the new node. The value returned by `child_builder_cb` becomes the return value of this function.
    ///
    /// For complex use cases where callbacks can get in the way, [`get_child_builder`](`NodeBuilder::get_child_builder`) may be more ergonomic.
    #[inline]
    pub fn build_child<R>(
        &mut self,
        val: T,
        child_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> R,
    ) -> R {
        let mut builder = self.get_child_builder();
        let ret = child_builder_cb(&mut builder);
        builder.finish(val);
        ret
    }

    /// Build a child node, whose value is the return value of the given closure, and add it to the tree as a child of the node
    /// that is being built by the current [`NodeBuilder`]. This is useful when you don't know the value of the child up front.
    /// 
    /// The parameter `child_builder_cb` is a callback function that is called exactly once. It is passed a `&mut `[`NodeBuilder`] that can be
    /// used to add children to the new node. The value returned by `child_builder_cb` becomes the value of the new node.
    /// 
    /// Returns a [`NodeRefMut`] to the added child node.
    /// 
    /// For complex use cases where callbacks can get in the way, [`get_child_builder`](`NodeBuilder::get_child_builder`) may be more ergonomic.
    /// 
    /// # Example:
    /// ```
    /// use packed_tree::{PackedTree, NodeRef, NodeBuilder};
    /// 
    /// // Assume you already have some kind of tree with floating point values, like this:
    /// let value_tree = PackedTree::new(1.2, |node_builder| {
    ///     node_builder.build_child(3.4, |node_builder| {
    ///         node_builder.add_child(5.6);
    ///     });
    ///     node_builder.add_child(7.8);
    /// });
    /// 
    /// // Build a tree from the previous tree,
    /// // where the value of a node is the sum of the values
    /// // of all the values of all the nodes below it (including itself).
    /// // Returns that sum.
    /// fn process_node(value_node: NodeRef<f64>, sum_node_builder: &mut NodeBuilder<f64>) -> f64 {
    ///     let mut sum = *value_node.val();
    ///     for value_child in value_node.children() {
    ///         let sum_child_node_ref = sum_node_builder.build_child_by_ret_val(|sum_child_builder| {
    ///             process_node(value_child, sum_child_builder)
    ///         });
    ///         sum += *sum_child_node_ref.val();
    ///     }
    ///     sum
    /// }
    /// 
    /// let sum_tree = PackedTree::new_by_ret_val(|node_builder| {
    ///     process_node(value_tree.root(), node_builder)
    /// });
    /// 
    /// assert_eq!(*sum_tree.root().val(), 1.2+3.4+5.6+7.8);
    /// ```
    #[inline]
    pub fn build_child_by_ret_val(
        &mut self,
        child_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> T,
    ) -> NodeRefMut<T> {
        let mut builder = self.get_child_builder();
        let val = child_builder_cb(&mut builder);
        builder.finish(val)
    }

    /// Add a child node with the given value to the tree as a child of the node that is being built by the current [`NodeBuilder`].
    /// 
    /// There is no way to add children to this new child node. Use [`build_child`](`NodeBuilder::build_child`)
    /// or [`get_child_builder`](`NodeBuilder::get_child_builder`) instead if that's what you want to do.
    /// 
    /// Returns a [`NodeRefMut`] to the added child node.
    #[inline]
    pub fn add_child(&mut self, val: T) -> NodeRefMut<T> {
        self.get_child_builder().finish(val)
    }
}

impl<'t, T> NodeDrain<'t, T> {
    /// Counts the number of descendants of this node (also counting the node itself) in O(1) time.
    #[inline(always)]
    pub fn num_descendants_incl_self(&self) -> usize {
        self.children.num_remaining_nodes_incl_descendants() + 1
    }

    /// Counts the number of descendants of this node (not counting the node itself) in O(1) time.
    #[inline(always)]
    pub fn num_descendants_excl_self(&self) -> usize {
        self.children.num_remaining_nodes_incl_descendants()
    }
}
