use std::convert::{From, TryFrom, AsRef};
use crate::*;

/// A `PackedTree` is a tree where all nodes are stored in a single `Vec` with only a single `usize` overhead per node.
/// It allows for fast creation, cache-friendly iteration (in pre-order or depth-first order),
/// and efficient storage of the tree.
/// 
/// A limitation of `PackedTree` is that it has to be created in one go, and does not allow for adding or removing
/// nodes after creation.
/// 
/// If you want to store multiple trees in the same `Vec`, see [`PackedForest`].
/// 
/// See the [module-level documentation](index.html) for more information.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct PackedTree<T> {
    forest: PackedForest<T>,
}

impl<T> PackedTree<T> {
    /// Create a new `PackedTree`.
    ///
    /// The parameter `val` is the value that the root node will have.
    ///
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a `&mut `[`NodeBuilder`] that can be
    /// used to add children to the tree.
    ///
    /// For more complex use cases, see [`new_by_ret_val`](PackedTree::new_by_ret_val) and
    /// [`try_new_from_forest`](PackedTree::try_new_from_forest).
    #[inline]
    pub fn new(root_val: T, node_builder_cb: impl FnOnce(&mut NodeBuilder<T>)) -> PackedTree<T> {
        let mut forest = PackedForest::new();
        forest.build_tree(root_val, node_builder_cb);
        PackedTree { forest }
    }

    /// Create a new `PackedTree`, where the root value is the return value of the given closure.
    ///
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a `&mut `[`NodeBuilder`] that can be
    /// used to add children to the tree. The return value of this closure is the value that the root node will have.
    ///
    /// This is useful when you don't know the value that the tree's root node will have before creating the tree.
    /// See [`NodeBuilder::build_child_by_ret_val`] for an example.
    ///
    /// For more complex use cases, see [`PackedTree::try_new_from_forest`].
    #[inline]
    pub fn new_by_ret_val(node_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> T) -> PackedTree<T> {
        let mut forest = PackedForest::new();
        forest.build_tree_by_ret_val(node_builder_cb);
        PackedTree { forest }
    }

    /// Create a new `PackedTree` from the given `Forest`. Returns `None` when the forest doesn't have exactly 1 tree.
    /// 
    /// In some cases, it is easier to build a [`PackedForest`] than a [`PackedTree`], for 2 reasons:
    ///   * [`PackedTree`] doesn't have the equivalent of the method [`PackedForest::get_tree_builder`].
    ///   * The methods constructing a [`PackedTree`] already return a [`PackedTree`], so they can't
    ///     also return the return value of the closure used to construct them, unlike [`PackedForest::build_tree`].
    /// 
    /// In those cases you can construct a [`PackedForest`], and then use this method to construct the
    /// [`PackedTree`]. Alternatively, [`std::convert::TryFrom`] is also possible.
    #[inline(always)]
    pub fn try_from_forest(forest: PackedForest<T>) -> Option<PackedTree<T>> {
        let mut iter = forest.iter_trees();
        match iter.next() {
            Some(_) => {
                if iter.next().is_none() {
                    Some(PackedTree {
                        forest
                    })
                } else {
                    None
                }
            },
            None => None
        }
    }

    /// Returns a [`NodeRef`] reference to the tree's root.
    #[inline(always)]
    pub fn root(&self) -> NodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    /// Returns a [`NodeRefMut`] mutable reference to the tree's root.
    #[inline(always)]
    pub fn root_mut(&mut self) -> NodeRefMut<T> {
        self.forest.iter_trees_mut().next().unwrap()
    }

    /// Converts `self` into a [`PackedTreeDrain`] which can then be used to drain the tree.
    /// 
    /// The reason for this slightly convoluted method is that the methods for draining
    /// a tree actually borrow the tree mutably rather than taking it by value,
    /// because it's actually possible to drain separate subtrees in parallel, but
    /// something needs to be responsible for actually owning the data until all of it
    /// is drained. That something in this case is the [`PackedTreeDrain`]
    #[inline(always)]
    pub fn drain(self) -> PackedTreeDrain<T> {
        PackedTreeDrain {
            forest: self.forest
        }
    }

    /// Iterate over all the values in all the nodes in this tree, in pre-order order.
    #[inline(always)]
    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<T>>, impl FnMut(&'a NodeData<T>) -> &'a T>
    {
        self.forest.iter_flattened()
    }

    /// Iterate over all the values in all the nodes in this tree mutably, in pre-order order.
    #[inline(always)]
    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::slice::IterMut<'a, NodeData<T>>,
        impl FnMut(&'a mut NodeData<T>) -> &'a mut T,
    > {
        self.forest.iter_flattened_mut()
    }

    /// Read-only view of the raw data.
    #[inline(always)]
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        self.forest.raw_data()
    }

    /// Returns how many nodes are currently in this tree in O(1) time.
    #[inline(always)]
    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

impl<T> TryFrom<PackedForest<T>> for PackedTree<T> {
    type Error = ();
    #[inline(always)]
    fn try_from(forest: PackedForest<T>) -> Result<Self, Self::Error> {
        match PackedTree::try_from_forest(forest) {
            Some(tree) => Ok(tree),
            None => Err(())
        }
    }
}

impl<T> AsRef<PackedForest<T>> for PackedTree<T> {
    #[inline(always)]
    fn as_ref(&self) -> &PackedForest<T> {
        &self.forest
    }
}

impl<T> From<PackedTree<T>> for PackedForest<T> {
    #[inline(always)]
    fn from(tree: PackedTree<T>) -> Self {
        tree.forest
    }
}

/// A [`PackedTree`] that is being drained.
/// See [`PackedTree::drain`].
pub struct PackedTreeDrain<T> {
    forest: PackedForest<T>,
}

impl<T> PackedTreeDrain<T> {
    /// Returns a [`NodeDrain`] that contains the value of the root node and a draining iterator
    /// of its children, or `None` if this tree has already been drained.
    #[inline(always)]
    pub fn drain_root(&mut self) -> Option<NodeDrain<T>> {
        self.forest.drain_trees().next()
    }

    /// Returns a draining iterator over all the values in all the nodes in this tree, in pre-order order.
    /// The iterator is empty if the tree has already been drained.
    /// 
    /// Dropping the iterator drops all the nodes in the forest that haven't been iterated over yet.
    /// 
    /// **WARNING:** Leaking the returned iterator without iterating over all of its values will leak the
    /// values that were not iterated over. They will still be removed from the tree though.
    #[inline(always)]
    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.forest.drain_flattened()
    }
}
