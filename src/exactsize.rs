use crate::*;
use std::convert::TryFrom;
use std::iter::{ExactSizeIterator, Iterator};

#[derive(Default,Eq,PartialEq,Hash,Clone)]
pub struct ExactSize<T> {
    val: T,
    num_children: usize,
}

/// A variant of [`PackedForest`] where the nodes keep track of how many direct children they have.
#[derive(Default, Eq, PartialEq, Hash, Clone)]
pub struct ExactSizePackedForest<T> {
    forest: PackedForest<ExactSize<T>>,
    num_trees: usize
}

impl<T> ExactSizePackedForest<T> {
    /// Create a new, empty [`ExactSizePackedForest`].
    /// 
    /// Note that [`ExactSizePackedForest`] implements [`Default`].
    #[inline(always)]
    pub fn new() -> ExactSizePackedForest<T> {
        ExactSizePackedForest {
            forest: PackedForest::new(),
            num_trees: 0
        }
    }

    /// Create a new [`ExactSizePackedForest`] with the specified capacity for the inner `Vec` which stores the nodes (see [`Vec::with_capacity`]).
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> ExactSizePackedForest<T> {
        ExactSizePackedForest {
            forest: PackedForest::with_capacity(capacity),
            num_trees: 0
        }
    }

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
        node_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> R,
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
        node_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> T,
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

    /// Get a [`ExactSizeNodeBuilder`] that can be used to build a tree that will be added to this forest.
    /// 
    /// See [`NodeBuilder`] for more information.
    #[inline]
    pub fn get_tree_builder(&mut self) -> ExactSizeNodeBuilder<T> {
        ExactSizeNodeBuilder {
            sub_node_builder: self.forest.get_tree_builder(),
            num_children: 0
        }
    }

    /// Returns an iterator that iterates over (a [`NodeRef`] to) all the trees in this forest.
    #[inline(always)]
    pub fn iter_trees(&self) -> ExactSizeNodeIter<T> {
        ExactSizeNodeIter {
            sub_iter: self.forest.iter_trees(),
            len: self.num_trees
        }
    }

    /// Returns an iterator that iterates over [`NodeRefMut`]s to all the trees in this forest.
    /// With this iterator you can change values of nodes in the tree (see [`NodeRefMut::val_mut`]),
    /// but you can't change the structure of the tree.
    #[inline(always)]
    pub fn iter_trees_mut(&mut self) -> ExactSizeNodeIterMut<T> {
        ExactSizeNodeIterMut {
            sub_iter: self.forest.iter_trees_mut(),
            len: self.num_trees
        }
    }

    /// Returns a draining iterator over the trees of this forest. The values returned by this iterator
    /// are [`NodeDrain`]s, a simple struct containing the public fields `val` (the value of the node) and
    /// `children`, another draining iterator over the children of the node.
    /// 
    /// After iterating or after dropping the iterator, the forest will be empty.
    /// 
    /// **WARNING:** if the [`NodeListDrain`] returned by this function is leaked (i.e. through [`std::mem::forget`])
    /// without iterating over all the values in it, then the values of the nodes that were not iterated over
    /// will also be leaked (their `drop` method won't be called). They will still be removed from the forest though.
    #[inline(always)]
    pub fn drain_trees(&mut self) -> ExactSizeNodeListDrain<'_, T> {
        ExactSizeNodeListDrain {
            sub_iter: self.forest.drain_trees(),
            len: self.num_trees
        }
    }

    /// Get a [`NodeRef`] to the node with the given index, or `None` if the index is out of bounds.
    /// 
    /// Nodes are indexed in pre-order ordering, i.e., in the order you would encounter
    /// them in a depth-first search. So the index of the first tree's root node is 0,
    /// the index of its first child (if any) is 1, the index of that first child's
    /// first child (if any) is 2, etc.
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<ExactSizeNodeRef<T>> {
        self.forest.get(index).map(|sub_ref| {
            ExactSizeNodeRef {
                sub_ref
            }
        })
    }

    /// Get a [`NodeRefMut`] to the node with the given index, or `None` if the index is out of bounds.
    /// 
    /// Nodes are indexed in pre-order ordering, i.e., in the order you would encounter
    /// them in a depth-first search. So the index of the first tree's root node is 0,
    /// the index of its first child (if any) is 1, the index of that first child's
    /// first child (if any) is 2, etc.
    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<ExactSizeNodeRefMut<T>> {
        self.forest.get_mut(index).map(|sub_ref| {
            ExactSizeNodeRefMut {
                sub_ref
            }
        })
    }

    /// Get a [`NodeRef`] to the node with the given index.
    /// 
    /// Does **not** check that the given index is in bounds, and is therefore unsafe.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> ExactSizeNodeRef<T> {
        ExactSizeNodeRef {
            sub_ref: self.forest.get_unchecked(index)
        }
    }

    /// Get a [`NodeRefMut`] to the node with the given index.
    /// 
    /// Does **not** check that the given index is in bounds, and is therefore unsafe.
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> ExactSizeNodeRefMut<T> {
        ExactSizeNodeRefMut {
            sub_ref: self.forest.get_unchecked_mut(index)
        }
    }

    /// Remove all nodes from the forest.
    #[inline]
    pub fn clear(&mut self) {
        self.forest.clear()
    }

    /// Iterate over all the values in all the nodes of all the trees in this forest, in pre-order order.
    #[inline(always)]
    pub fn iter_flattened<'t>(
        &'t self,
    ) -> std::iter::Map<std::slice::Iter<'t, NodeData<ExactSize<T>>>, impl FnMut(&'t NodeData<ExactSize<T>>) -> &'t T>
    {
        self.forest.raw_data().iter().map(|node_data| &node_data.val().val)
    }

    /// Iterate mutably over all the values in all the nodes of all the trees in this forest, in pre-order order.
    #[inline(always)]
    pub fn iter_flattened_mut<'t>(
        &'t mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::slice::IterMut<'t, NodeData<ExactSize<T>>>,
            impl FnMut(&'t mut NodeData<ExactSize<T>>) -> &'t mut ExactSize<T>,
        >,
        impl FnMut(&'t mut ExactSize<T>) -> &'t mut T,
    > {
        self.forest.iter_flattened_mut().map(|node_data| &mut node_data.val)
    }

    /// Returns a draining iterator over all the values in all the nodes of all the trees in this forest, in pre-order order.
    /// 
    /// Dropping the iterator drops all the nodes in the forest that haven't been iterated over yet.
    /// 
    /// **WARNING:** Leaking the returned iterator without iterating over all of its values will leak the
    /// values that were not iterated over. They will still be removed from the tree though.
    #[inline(always)]
    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::vec::Drain<NodeData<ExactSize<T>>>,
            impl FnMut(NodeData<ExactSize<T>>) -> ExactSize<T>,
        >,
        impl FnMut(ExactSize<T>) -> T,
    > {
        self.forest
            .drain_flattened()
            .map(|exact_size| exact_size.val)
    }

    /// Returns a read-only view over the raw data stored internally by this `PackedForest`.
    /// This is not really recommended to be used except for very advanced use cases.
    #[inline(always)]
    pub fn raw_data(&self) -> &Vec<NodeData<ExactSize<T>>> {
        self.forest.raw_data()
    }

    /// Returns how many nodes are currently in all the trees in this forest in O(1) time.
    #[inline(always)]
    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

/// `NodeBuilder` is a struct that lets you add children to a node that is currently being added
/// to a [`PackedTree`](crate::PackedTree) or a [`PackedForest`].
/// 
/// See [`PackedTree::new`](crate::PackedTree::new), [`PackedForest::build_tree`], [`PackedForest::get_tree_builder`], etc.
/// 
// IMPLEMENTATION NOTES:
// The fields of the struct are:
// - forest: mutable ref to the forest to which we're adding this node.
// - index: the index where the node that we're adding will end up in self.forest.data
// - subtree_size: the number of elements in the subtree that has this node as root,
//   not counting children that haven't had finish() called on their NodeBuilder instances yet.
// - parent_subtree_size: mutable reference to the parent's Node subtree_size (or None if no parent)
//
// INVARIANTS:
// 1. The values in the Vec forest.data between indices index+1 (inclusive) and index+subtree_size (exclusive)
//    are initialized, valid, and within the capacity of the Vec but outside of the len of the Vec.
// 2. If this node has a parent, self.index must be equal to parent.index + parent.subtree_size,
//    otherwise index must be equal to forest.data.len().
pub struct ExactSizeNodeBuilder<'a, T> {
    sub_node_builder: NodeBuilder<'a,ExactSize<T>>,
    num_children: usize
}

impl<'a, T> ExactSizeNodeBuilder<'a, T> {
    /// Returns the index of the node that is being built.
    /// 
    /// See also [`PackedForest::get`] and [`PackedForest::get_mut`].
    #[inline(always)]
    pub fn index(&self) -> usize {
        self.sub_node_builder.index()
    }

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
        child_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> R,
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
        child_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> T,
    ) -> ExactSizeNodeRefMut<T> {
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
    pub fn add_child(&mut self, val: T) -> ExactSizeNodeRefMut<T> {
        self.get_child_builder().finish(val)
    }

    #[inline]
    pub fn get_child_builder<'b>(&'b mut self) -> ExactSizeNodeBuilder<'b, T> {
        ExactSizeNodeBuilder {
            sub_node_builder: self.sub_node_builder.get_child_builder(),
            num_children: 0
        }
    }

    /// Finish building the node that this [`NodeBuilder`] was building, giving it its value
    /// and adding its nodes to the tree, forest or the parent [`NodeBuilder`].
    /// Returns a [`NodeRefMut`] to the node that was added.
    /// 
    /// This method doesn't need to be (and in fact can't be) called when adding nodes through
    /// methods like [`build_child`](NodeBuilder::build_child) or
    /// [`build_child_by_ret_val`](NodeBuilder::build_child_by_ret_val).
    /// It is called automatically. It only needs to be called if the [`NodeBuilder`]
    /// was obtained through methods like [`get_child_builder`](NodeBuilder::get_child_builder) or
    /// [`PackedForest::get_tree_builder`].
    /// 
    /// Dropping a [`NodeBuilder`] without calling `finish` drops all the nodes that have been
    /// added to it without adding them to the tree or forest. Leaking a [`NodeBuilder`]
    /// (i.e. through [`std::mem::forget`]) causes all the nodes added to it to be leaked instead
    /// (their `drop` method won't be called).
    /// 
    /// See [`get_child_builder`](NodeBuilder::get_child_builder) for an example of how to use this.
    #[inline]
    pub fn finish(self, val: T) -> ExactSizeNodeRefMut<'a,T> {
        ExactSizeNodeRefMut {
            sub_ref: self.sub_node_builder.finish(ExactSize {
                val,
                num_children: self.num_children
            })
        }
    }
}

/// Iterates a list of nodes in a [`PackedForest`] or [`PackedTree`], usually the list
/// of children of a node, or the list of root nodes in a [`PackedForest`].
/// 
/// See e.g. [`PackedForest::iter_trees`] and [`NodeRef::children`].
pub struct ExactSizeNodeIter<'t, T> {
    sub_iter: NodeIter<'t, ExactSize<T>>,
    len: usize
}

// Not using #[derive(Copy)] because it adds the T:Copy bound, which is unnecessary
impl<'t,T> Copy for ExactSizeNodeIter<'t,T> {}

// Not using #[derive(Clone)] because it adds the T:Clone bound, which is unnecessary
impl<'t, T> Clone for ExactSizeNodeIter<'t, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'t, T> ExactSizeNodeIter<'t, T> {
    /// Returns the number of nodes (also counting all descendants) remaining in this iterator in O(1) time.
    #[inline(always)]
    pub fn num_remaining_nodes_incl_descendants(&self) -> usize {
        self.sub_iter.num_remaining_nodes_incl_descendants()
    }
}

impl<'t, T> Iterator for ExactSizeNodeIter<'t, T> {
    type Item = ExactSizeNodeRef<'t, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.sub_iter.next().map(|sub_ref| {
            ExactSizeNodeRef {
                sub_ref
            }
        })
    }

    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeIter<'t, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
}

/// A shared reference to a node in a [`PackedForest`] or [`PackedTree`].
pub struct ExactSizeNodeRef<'t, T> {
    sub_ref: NodeRef<'t, ExactSize<T>>
}

// Not using #[derive(Copy)] because it adds the T:Copy bound, which is unnecessary
impl<'t,T> Copy for ExactSizeNodeRef<'t,T> {}

// Not using #[derive(Clone)] because it adds the T:Clone bound, which is unnecessary
impl<'t,T> Clone for ExactSizeNodeRef<'t,T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'t, T> ExactSizeNodeRef<'t, T> {
    /// Returns an iterator to the children of this node.
    #[inline(always)]
    pub fn children(&self) -> ExactSizeNodeIter<'t, T> {
        ExactSizeNodeIter {
            sub_iter: self.sub_ref.children(),
            len: self.sub_ref.val().num_children
        }
    }

    /// Returns a reference to the value of this node.
    #[inline(always)]
    pub fn val(&self) -> &T {
        &self.sub_ref.val().val
    }

    #[inline(always)]
    pub fn num_children(&self) -> usize {
        self.sub_ref.val().num_children
    }

    /// Counts the number of descendants of this node (also counting the node itself) in O(1) time.
    #[inline(always)]
    pub fn num_descendants_incl_self(&self) -> usize {
        self.sub_ref.num_descendants_incl_self()
    }

    /// Counts the number of descendants of this node (not counting the node itself) in O(1) time.
    #[inline(always)]
    pub fn num_descendants_excl_self(&self) -> usize {
        self.sub_ref.num_descendants_excl_self()
    }
}

/// A mutable reference to a node in a [`PackedForest`] or [`PackedTree`].
pub struct ExactSizeNodeIterMut<'t, T> {
    sub_iter: NodeIterMut<'t, ExactSize<T>>,
    len: usize
}

impl<'t, T> Iterator for ExactSizeNodeIterMut<'t, T> {
    type Item = ExactSizeNodeRefMut<'t, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.sub_iter.next().map(|sub_ref| {
            ExactSizeNodeRefMut {
                sub_ref
            }
        })
    }
    
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeIterMut<'t, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
}

impl<'t, T> ExactSizeNodeIterMut<'t, T> {
    /// Reborrow this [`NodeIterMut`] as a [`NodeIter`].
    #[inline(always)]
    pub fn reborrow_shared(&self) -> ExactSizeNodeIter<T> {
        ExactSizeNodeIter {
            sub_iter: self.sub_iter.reborrow_shared(),
            len: self.len
        }
    }

    /// Returns the number of nodes (also counting all descendants) remaining in this iterator in O(1) time.
    #[inline(always)]
    pub fn num_remaining_nodes_incl_descendants(&self) -> usize {
        self.sub_iter.num_remaining_nodes_incl_descendants()
    }
}

impl<'t,T> From<ExactSizeNodeIterMut<'t,T>> for ExactSizeNodeIter<'t,T> {
    #[inline(always)]
    fn from(val: ExactSizeNodeIterMut<'t,T>) -> Self {
        ExactSizeNodeIter {
            sub_iter: val.sub_iter.into(),
            len: val.len
        }
    }
}

pub struct ExactSizeNodeRefMut<'t, T> {
    sub_ref: NodeRefMut<'t, ExactSize<T>>
}

impl<'t, T> ExactSizeNodeRefMut<'t, T> {
    /// Returns an iterator to the children of this node.
    /// 
    /// The difference between this and [`NodeRefMut::children`] is that this method
    /// consumes self and is therefore able to return a broader lifetime.
    #[inline(always)]
    pub fn into_children(self) -> ExactSizeNodeIterMut<'t, T> {
        let len = self.sub_ref.val().num_children;
        ExactSizeNodeIterMut {
            sub_iter: self.sub_ref.into_children(),
            len
        }
    }

    /// Returns an iterator to the children of this node.
    /// 
    /// The difference between this and [`NodeRefMut::into_children`] is that this method
    /// reborrows self, so the lifetime of the returned iterator is that of the
    /// mutable reference passed to this function.
    #[inline(always)]
    pub fn children(&mut self) -> ExactSizeNodeIterMut<T> {
        let len = self.sub_ref.val().num_children;
        ExactSizeNodeIterMut {
            sub_iter: self.sub_ref.children(),
            len
        }
    }

    /// Returns a shared reference to the value of this node.
    #[inline(always)]
    pub fn val(&self) -> &T {
        &self.sub_ref.val().val
    }

    /// Returns a mutable reference to the value of this node.
    #[inline(always)]
    pub fn val_mut(&mut self) -> &mut T {
        &mut self.sub_ref.val_mut().val
    }

    #[inline(always)]
    pub fn num_children(&self) -> usize {
        self.sub_ref.val().num_children
    }

    /// Reborrow this [`NodeRefMut`] as a [`NodeRef`].
    #[inline(always)]
    pub fn reborrow_shared(&self) -> ExactSizeNodeRef<T> {
        ExactSizeNodeRef {
            sub_ref: self.sub_ref.reborrow_shared()
        }
    }

    /// Counts the number of descendants of this node (also counting the node itself) in O(1) time.
    #[inline(always)]
    pub fn num_descendants_incl_self(&self) -> usize {
        self.sub_ref.num_descendants_incl_self()
    }

    /// Counts the number of descendants of this node (not counting the node itself) in O(1) time.
    #[inline(always)]
    pub fn num_descendants_excl_self(&self) -> usize {
        self.sub_ref.num_descendants_excl_self()
    }
}

impl<'t,T> From<ExactSizeNodeRefMut<'t,T>> for ExactSizeNodeRef<'t,T> {
    #[inline(always)]
    fn from(val: ExactSizeNodeRefMut<'t,T>) -> Self {
        ExactSizeNodeRef {
            sub_ref: val.sub_ref.into()
        }
    }
}

/// A draining iterator of a list of nodes in a [`PackedForest`] or [`PackedTree`].
/// 
/// When this iterator is dropped, the nodes remaining in the iterator will be dropped.
/// If this iterator is leaked instead (through e.g. [`std::mem::forget`]),
/// these nodes also will be leaked instead.
/// 
/// See [`PackedForest::drain_trees`] and [`PackedTree::drain`].
pub struct ExactSizeNodeListDrain<'t, T> {
    sub_iter: NodeListDrain<'t, ExactSize<T>>,
    len: usize
}

impl<'t, T> Iterator for ExactSizeNodeListDrain<'t, T> {
    type Item = ExactSizeNodeDrain<'t, T>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.sub_iter.next().map(|sub_drain| {
            ExactSizeNodeDrain {
                val: sub_drain.val.val,
                children: ExactSizeNodeListDrain {
                    sub_iter: sub_drain.children,
                    len: sub_drain.val.num_children
                }
            }
        })
    }
    
    #[inline(always)]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'t, T> ExactSizeIterator for ExactSizeNodeListDrain<'t, T> {
    #[inline(always)]
    fn len(&self) -> usize {
        self.len
    }
}

impl<'t, T> ExactSizeNodeListDrain<'t, T> {
    /// Returns the number of nodes (also counting all descendants) remaining in this iterator in O(1) time.
    #[inline(always)]
    pub fn num_remaining_nodes_incl_descendants(&self) -> usize {
        self.sub_iter.num_remaining_nodes_incl_descendants()
    }
}

/// A node in a [`PackedForest`] or [`PackedTree`] that is being drained.
/// You can move out its fields `val` and `children` (which is a [`NodeListDrain`]) directly.
pub struct ExactSizeNodeDrain<'t, T> {
    pub val: T,
    pub children: ExactSizeNodeListDrain<'t, T>
}

impl<'t, T> ExactSizeNodeDrain<'t, T> {
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
pub struct ExactSizePackedTree<T> {
    forest: ExactSizePackedForest<T>,
}

impl<T> ExactSizePackedTree<T> {
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
    pub fn new(root_val: T, node_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>)) -> ExactSizePackedTree<T> {
        let mut forest = ExactSizePackedForest::new();
        forest.build_tree(root_val, node_builder_cb);
        ExactSizePackedTree { forest }
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
    pub fn new_by_ret_val(node_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> T) -> ExactSizePackedTree<T> {
        let mut forest = ExactSizePackedForest::new();
        forest.build_tree_by_ret_val(node_builder_cb);
        ExactSizePackedTree { forest }
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
    pub fn try_from_forest(forest: ExactSizePackedForest<T>) -> Option<ExactSizePackedTree<T>> {
        let mut iter = forest.iter_trees();
        match iter.next() {
            Some(_) => {
                if iter.next().is_none() {
                    Some(ExactSizePackedTree {
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
    pub fn root(&self) -> ExactSizeNodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    /// Returns a [`NodeRefMut`] mutable reference to the tree's root.
    #[inline(always)]
    pub fn root_mut(&mut self) -> ExactSizeNodeRefMut<T> {
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
    pub fn drain(self) -> ExactSizePackedTreeDrain<T> {
        ExactSizePackedTreeDrain {
            forest: self.forest
        }
    }

    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<ExactSize<T>>>, impl FnMut(&'a NodeData<ExactSize<T>>) -> &'a T> {
        self.forest.iter_flattened()
    }

    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::slice::IterMut<'a, NodeData<ExactSize<T>>>,
            impl FnMut(&'a mut NodeData<ExactSize<T>>) -> &'a mut ExactSize<T>,
        >,
        impl FnMut(&'a mut ExactSize<T>) -> &'a mut T,
    > {
        self.forest.iter_flattened_mut()
    }

    /// Read-only view of the raw data.
    #[inline(always)]
    pub fn raw_data(&self) -> &Vec<NodeData<ExactSize<T>>> {
        self.forest.raw_data()
    }

    /// Returns how many nodes are currently in this tree in O(1) time.
    #[inline(always)]
    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

impl<T> TryFrom<ExactSizePackedForest<T>> for ExactSizePackedTree<T> {
    type Error = ();
    #[inline(always)]
    fn try_from(forest: ExactSizePackedForest<T>) -> Result<Self, Self::Error> {
        match ExactSizePackedTree::try_from_forest(forest) {
            Some(tree) => Ok(tree),
            None => Err(())
        }
    }
}

impl<T> AsRef<ExactSizePackedForest<T>> for ExactSizePackedTree<T> {
    #[inline(always)]
    fn as_ref(&self) -> &ExactSizePackedForest<T> {
        &self.forest
    }
}

impl<T> From<ExactSizePackedTree<T>> for ExactSizePackedForest<T> {
    #[inline(always)]
    fn from(tree: ExactSizePackedTree<T>) -> Self {
        tree.forest
    }
}

/// A [`PackedTree`] that is being drained.
/// See [`PackedTree::drain`].
pub struct ExactSizePackedTreeDrain<T> {
    forest: ExactSizePackedForest<T>,
}

impl<T> ExactSizePackedTreeDrain<T> {
    /// Returns a [`NodeDrain`] that contains the value of the root node and a draining iterator
    /// of its children, or `None` if this tree has already been drained.
    #[inline(always)]
    pub fn drain_root(&mut self) -> Option<ExactSizeNodeDrain<T>> {
        self.forest.drain_trees().next()
    }

    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<
        std::iter::Map<
            std::vec::Drain<NodeData<ExactSize<T>>>,
            impl FnMut(NodeData<ExactSize<T>>) -> ExactSize<T>,
        >,
        impl FnMut(ExactSize<T>) -> T,
    > {
        self.forest.drain_flattened()
    }
}
