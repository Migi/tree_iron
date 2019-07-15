use crate::*;
use std::convert::TryFrom;
use std::iter::{ExactSizeIterator, Iterator};

/// The data that an [`ExactSizePackedForest`] stores per node: a value (a [`NodeData`]), and a `usize num_children`.
#[derive(Default,Eq,PartialEq,Hash,Clone)]
pub struct ExactSize<T> {
    val: T,
    num_children: usize,
}

impl<T> ExactSize<T> {
    /// Get the value.
    #[inline(always)]
    pub fn val(&self) -> &T {
        &self.val
    }
    
    /// Get the number of direct children of this node.
    #[inline(always)]
    pub fn num_children(&self) -> usize {
        self.num_children
    }
}

/// A variant of [`PackedForest`] that keeps track of how many children each node has.
/// 
/// That allows iterators of a node's children to be [`ExactSizeIterator`]s in addition to being regular [`Iterator`]s.
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
    /// See [`PackedForest::build_tree`].
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
    /// See [`PackedForest::build_tree_by_ret_val`].
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
    /// See [`PackedForest::get_tree_builder`] and [`NodeBuilder`] for more information.
    #[inline]
    pub fn get_tree_builder(&mut self) -> ExactSizeNodeBuilder<T> {
        ExactSizeNodeBuilder {
            sub_node_builder: self.forest.get_tree_builder(),
            num_children: 0
        }
    }

    /// Returns an iterator that iterates over all the trees in this forest.
    #[inline(always)]
    pub fn iter_trees(&self) -> ExactSizeNodeIter<T> {
        ExactSizeNodeIter {
            sub_iter: self.forest.iter_trees(),
            len: self.num_trees
        }
    }

    /// Returns an iterator that iterates mutably over all the trees in this forest.
    /// With this iterator you can change values of nodes in the tree (see [`ExactSizeNodeRefMut::val_mut`]),
    /// but you can't change the structure of the tree.
    #[inline(always)]
    pub fn iter_trees_mut(&mut self) -> ExactSizeNodeIterMut<T> {
        ExactSizeNodeIterMut {
            sub_iter: self.forest.iter_trees_mut(),
            len: self.num_trees
        }
    }

    /// Returns a draining iterator over the trees of this forest.
    /// 
    /// See [`PackedForest::drain_trees`].
    #[inline(always)]
    pub fn drain_trees(&mut self) -> ExactSizeNodeListDrain<'_, T> {
        ExactSizeNodeListDrain {
            sub_iter: self.forest.drain_trees(),
            len: self.num_trees
        }
    }

    /// Get an [`ExactSizeNodeRef`] to the node with the given index, or `None` if the index is out of bounds.
    /// 
    /// See [`PackedForest::get`].
    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<ExactSizeNodeRef<T>> {
        self.forest.get(index).map(|sub_ref| {
            ExactSizeNodeRef {
                sub_ref
            }
        })
    }

    /// Get an [`ExactSizeNodeRefMut`] to the node with the given index, or `None` if the index is out of bounds.
    /// 
    /// See [`PackedForest::get_mut`].
    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<ExactSizeNodeRefMut<T>> {
        self.forest.get_mut(index).map(|sub_ref| {
            ExactSizeNodeRefMut {
                sub_ref
            }
        })
    }

    /// Get an [`ExactSizeNodeRef`] to the node with the given index.
    /// 
    /// Does **not** check that the given index is in bounds, and is therefore unsafe.
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> ExactSizeNodeRef<T> {
        ExactSizeNodeRef {
            sub_ref: self.forest.get_unchecked(index)
        }
    }

    /// Get an [`ExactSizeNodeRefMut`] to the node with the given index.
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

    /// Returns a read-only view over the raw data stored internally by this [`ExactSizePackedForest`].
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

/// A struct that lets you add children to a node that is currently being added to a [`ExactSizePackedTree`] or a [`ExactSizePackedForest`].
/// 
/// See [`NodeBuilder`] for more information.
pub struct ExactSizeNodeBuilder<'a, T> {
    sub_node_builder: NodeBuilder<'a,ExactSize<T>>,
    num_children: usize
}

impl<'a, T> ExactSizeNodeBuilder<'a, T> {
    /// Returns the index of the node that is being built.
    /// 
    /// See also [`ExactSizePackedForest::get`] and [`ExactSizePackedForest::get_mut`].
    #[inline(always)]
    pub fn index(&self) -> usize {
        self.sub_node_builder.index()
    }

    /// Build a child node with the given value, and add it to the tree as a child of the node
    /// that is being built by the current [`ExactSizeNodeBuilder`].
    ///
    /// See [`NodeBuilder::build_child`].
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
    /// that is being built by the current [`ExactSizeNodeBuilder`]. This is useful when you don't know the value of the child up front.
    ///
    /// See [`NodeBuilder::build_child_by_ret_val`].
    #[inline]
    pub fn build_child_by_ret_val(
        &mut self,
        child_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> T,
    ) -> ExactSizeNodeRefMut<T> {
        let mut builder = self.get_child_builder();
        let val = child_builder_cb(&mut builder);
        builder.finish(val)
    }

    /// Add a child node with the given value to the tree as a child of the node that is being built by the current [`ExactSizeNodeBuilder`].
    /// 
    /// See [`NodeBuilder::add_child`].
    #[inline]
    pub fn add_child(&mut self, val: T) -> ExactSizeNodeRefMut<T> {
        self.get_child_builder().finish(val)
    }

    /// Get an [`ExactSizeNodeBuilder`] that builds a child that will be added as a child of the node
    /// that is being built by the current [`ExactSizeNodeBuilder`].
    ///
    /// See [`NodeBuilder::get_child_builder`].
    #[inline]
    pub fn get_child_builder<'b>(&'b mut self) -> ExactSizeNodeBuilder<'b, T> {
        ExactSizeNodeBuilder {
            sub_node_builder: self.sub_node_builder.get_child_builder(),
            num_children: 0
        }
    }

    /// Finish building the node that this [`ExactSizeNodeBuilder`] was building, giving it its value
    /// and adding its nodes to the tree, forest or the parent [`ExactSizeNodeBuilder`].
    /// Returns a [`NodeRefMut`] to the node that was added.
    ///
    /// See [`NodeBuilder::finish`].
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

/// Iterates a list of nodes in an [`ExactSizePackedForest`] or [`ExactSizePackedTree`].
/// 
/// See [`NodeIter`].
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

/// A shared reference to a node in an [`ExactSizePackedForest`] or [`ExactSizePackedTree`].
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

    /// Returns the number of children of this node.
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

/// A mutable reference to a node in an [`ExactSizePackedForest`] or [`ExactSizePackedTree`].
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
    /// Reborrow this [`ExactSizeNodeIterMut`] as a [`ExactSizeNodeIter`].
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

/// A mutable reference to a node in an [`ExactSizePackedTree`] or [`ExactSizePackedForest`].
pub struct ExactSizeNodeRefMut<'t, T> {
    sub_ref: NodeRefMut<'t, ExactSize<T>>
}

impl<'t, T> ExactSizeNodeRefMut<'t, T> {
    /// Returns an iterator to the children of this node.
    /// 
    /// The difference between this and [`ExactSizeNodeRefMut::children`] is that this method
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
    /// The difference between this and [`ExactSizeNodeRefMut::into_children`] is that this method
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

    /// Returns the number of children of this node.
    #[inline(always)]
    pub fn num_children(&self) -> usize {
        self.sub_ref.val().num_children
    }

    /// Reborrow this [`ExactSizeNodeRefMut`] as an [`ExactSizeNodeRef`].
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

/// A draining iterator of a list of nodes in an [`ExactSizePackedForest`] or [`ExactSizePackedTree`].
/// 
/// See e.g. [`NodeListDrain`] and [`ExactSizePackedTree::drain`].
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

/// A node in an [`ExactSizePackedForest`] or [`ExactSizePackedTree`] that is being drained.
/// You can move out its fields `val` and `children` (which is an [`ExactSizeNodeListDrain`]) directly.
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

/// A variant of [`PackedTree`] that keeps track of the number of children of each node.
/// 
/// This allows iterators of the children of a node to be [`ExactSizeIterator`]s in addition to being regular [`Iterator`]s.
#[derive(Eq, PartialEq, Hash, Clone)]
pub struct ExactSizePackedTree<T> {
    forest: ExactSizePackedForest<T>,
}

impl<T> ExactSizePackedTree<T> {
    /// Create a new `ExactSizePackedTree`.
    ///
    /// See [`PackedTree::new`].
    #[inline]
    pub fn new(root_val: T, node_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>)) -> ExactSizePackedTree<T> {
        let mut forest = ExactSizePackedForest::new();
        forest.build_tree(root_val, node_builder_cb);
        ExactSizePackedTree { forest }
    }

    /// Create a new `ExactSizePackedTree`, where the root value is the return value of the given closure.
    ///
    /// See [`PackedTree::new_by_ret_val`].
    #[inline]
    pub fn new_by_ret_val(node_builder_cb: impl FnOnce(&mut ExactSizeNodeBuilder<T>) -> T) -> ExactSizePackedTree<T> {
        let mut forest = ExactSizePackedForest::new();
        forest.build_tree_by_ret_val(node_builder_cb);
        ExactSizePackedTree { forest }
    }

    /// Create a new `ExactSizePackedTree` from the given [`ExactSizePackedForest`]. Returns `None` when the forest doesn't have exactly 1 tree.
    ///
    /// See [`PackedTree::try_from_forest`].
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

    /// Returns an [`ExactSizeNodeRef`] reference to the tree's root.
    #[inline(always)]
    pub fn root(&self) -> ExactSizeNodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    /// Returns an [`ExactSizeNodeRefMut`] mutable reference to the tree's root.
    #[inline(always)]
    pub fn root_mut(&mut self) -> ExactSizeNodeRefMut<T> {
        self.forest.iter_trees_mut().next().unwrap()
    }

    /// Converts `self` into a [`ExactSizePackedTreeDrain`] which can then be used to drain the tree.
    /// 
    /// See [`PackedTree::drain`].
    #[inline(always)]
    pub fn drain(self) -> ExactSizePackedTreeDrain<T> {
        ExactSizePackedTreeDrain {
            forest: self.forest
        }
    }

    /// Iterate over all the values in all the nodes in this tree, in pre-order order.
    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<ExactSize<T>>>, impl FnMut(&'a NodeData<ExactSize<T>>) -> &'a T> {
        self.forest.iter_flattened()
    }

    /// Iterate over all the values in all the nodes in this tree mutably, in pre-order order.
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

/// An [`ExactSizePackedTree`] that is being drained. See [`ExactSizePackedTree::drain`].
pub struct ExactSizePackedTreeDrain<T> {
    forest: ExactSizePackedForest<T>,
}

impl<T> ExactSizePackedTreeDrain<T> {
    /// Returns an [`ExactSizeNodeDrain`] that contains the value of the root node and a draining iterator
    /// of its children, or `None` if this tree has already been drained.
    #[inline(always)]
    pub fn drain_root(&mut self) -> Option<ExactSizeNodeDrain<T>> {
        self.forest.drain_trees().next()
    }

    /// Returns a draining iterator over all the values in all the nodes in this tree, in pre-order order.
    /// The iterator is empty if the tree has already been drained.
    /// 
    /// See [`PackedTreeDrain::drain_flattened`].
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
