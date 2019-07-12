// TODO: rename "an"
// TODO: search all instances of "| mut"
// TODO: rename "|node|" in examples/tests with "|node_builder|"
// TODO: indexing
// TODO: return NodeRefMut on creation
// TODO: check safety of overflow
// TODO: #[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug, Hash)]?
// TODO: update dep versions
// TODO: clippy
// TODO: tests for leaks on panic
// TODO: Sync/Send on pointer-owning types
// TODO: #[inline]

// core.rs contains all the unsafe code.
// It should be kept as small as possible.
// No bugs outside of core.rs should lead to memory unsafety.
use std::iter::Iterator;
use std::num::NonZeroUsize;

/// Split off the first n elements of the pointed-to slice, modifying it.
/// Does *not* check that n <= len.
/// Implementation is similar to std::slice::split_at_mut.
#[inline(always)]
unsafe fn slice_split_off_first_n_unchecked<'a,T>(slice_ref: &mut &'a [T], n: usize) -> &'a [T] {
    let len = slice_ref.len();
    let ptr = slice_ref.as_ptr();

    debug_assert!(n <= len);

    *slice_ref = std::slice::from_raw_parts(ptr.add(n), len - n);
    std::slice::from_raw_parts(ptr, n)
}

/// Split off the first n elements of the pointed-to slice, modifying it.
/// Does *not* check that n <= len.
/// Implementation is similar to std::slice::split_at_mut
#[inline(always)]
unsafe fn slice_split_off_first_n_unchecked_mut<'a,T>(slice_ref: &mut &'a mut [T], n: usize) -> &'a mut [T] {
    let len = slice_ref.len();
    let ptr = slice_ref.as_mut_ptr();

    debug_assert!(n <= len);

    *slice_ref = std::slice::from_raw_parts_mut(ptr.add(n), len - n);
    std::slice::from_raw_parts_mut(ptr, n)
}

/// Split off the first element of the slice.
/// Does *not* check that the slice isn't empty.
#[inline(always)]
unsafe fn slice_split_first_unchecked<T>(slice: &[T]) -> (&T,&[T]) {
    debug_assert!(slice.len() > 0);
    (slice.get_unchecked(0),slice.get_unchecked(1..))
}

/// Split off the first element of the slice.
/// Does *not* check that the slice isn't empty.
#[inline(always)]
unsafe fn slice_split_first_unchecked_mut<T>(slice: &mut [T]) -> (&mut T,&mut [T]) {
    let len = slice.len();
    let ptr = slice.as_mut_ptr();
    
    debug_assert!(len > 0);

    (slice.get_unchecked_mut(0),std::slice::from_raw_parts_mut(ptr.add(1), len - 1))
}

/// An `IronedForest` is a list of trees, all stored in a single `Vec` with only 1 `usize` overhead per node.
/// This allows for fast and cache-friendly iteration (in pre-order or depth-first order) and efficient storage of the trees.
///
/// As opposed to [`IronedTree`], where you can never modify the structure once it's created,
/// you can add trees to an [`IronedForest`] after it's created (but you can't modify their structure).
///
/// # Example
/// ```
/// use tree_iron::{IronedForest, NodeRef};
///
/// // Create the forest
/// let mut forest = IronedForest::new();
///
/// // Add two trees
/// forest.build_tree("node 1", |mut node| {
/// 	node.add_child("node 1.1");
/// 	node.build_child("node 1.2", |mut node| {
/// 		node.add_child("node 1.2.1");
/// 	});
/// });
/// forest.build_tree("node 2", |mut node| {
///     node.add_child("node 2.1");
/// });
///
/// // Iterate it, counting the number of nodes
/// fn count_num_nodes(node: NodeRef<&'static str>) -> usize {
/// 	let mut result = 1;
/// 	for child in node.children() {
/// 		result += count_num_nodes(child);
/// 	}
/// 	result
/// }
///
/// let num_nodes_in_each_tree : Vec<_> = forest
///     .iter_trees()
///     .map(|root| count_num_nodes(root))
///     .collect();
///
/// assert_eq!(num_nodes_in_each_tree, [4, 2]);
/// ```
///
/// See the [module-level documentation](index.html) for more.
///
// =============== IMPLEMENTATION SAFETY NOTES ===================
//
// TODO
#[derive(Default)]
pub struct IronedForest<T> {
    data: Vec<NodeData<T>>,
}

impl<T> IronedForest<T> {
    /// Create a new [`IronedForest`].
    #[inline(always)]
    pub fn new() -> IronedForest<T> {
        IronedForest {
            data: Vec::new(),
        }
    }

    /// Create a new [`IronedForest`] with the specified capacity for the inner `Vec` which stores the nodes (see [`Vec::with_capacity`]).
    #[inline(always)]
    pub fn with_capacity(capacity: usize) -> IronedForest<T> {
        IronedForest {
            data: Vec::with_capacity(capacity),
        }
    }

    /// Build a tree with the given root value, and add it to the forest.
    /// 
    /// The parameter `root_val` is the value that the root of the tree will have.
    /// 
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a `[NodeBuilder]` that can be
    /// used to add nodes to the root node. The value returned by `node_builder_cb` becomes the return value of this function.
    /// 
    /// For complex use cases where callbacks can get in the way, [`get_tree_builder`](`IronedForest::get_tree_builder`) may be more ergonomic.
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

    /// Build a tree, where values of nodes come from return values of the given closure, and add it to the forest.
    /// 
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a [`NodeBuilder`] that can be
    /// used to add nodes to the root node. The value returned by `node_builder_cb` becomes the return value of this function.
    /// 
    /// For complex use cases where callbacks can get in the way, [`get_tree_builder`](`IronedForest::get_tree_builder`) may be more ergonomic.
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

    /// Get a [`NodeBuilder`] that can be used to build a tree that will be added to this forest.
    /// 
    /// The [`NodeBuilder`] borrows the forest mutably, so you can't do anything with the forest until you're
    /// done building the tree.
    /// 
    /// For simple use cases, using [`build_tree`](`IronedForest::build_tree`) is probably more ergonomic.
    /// 
    /// # Example:
    /// ```
    /// use tree_iron::{IronedTree, NodeRef, NodeBuilder};
    /// 
    /// // Assume you already have some kind of tree with floating point values, like this:
    /// let value_tree = IronedTree::new(1.2, |node| {
    ///     node.build_child(3.4, |node| {
    ///         node.add_child(5.6);
    ///     });
    ///     node.add_child(7.8);
    /// });
    /// 
    /// // Build a tree from the previous tree,
    /// // where the value of a node is the sum of the values
    /// // of all the values of all the nodes below it (including itself).
    /// // Returns that sum.
    /// fn process_node(value_node: NodeRef<f64>, sum_node_builder: &mut NodeBuilder<f64>) -> f64 {
    ///     let mut sum = *value_node.val();
    ///     for value_child in value_node.children() {
    ///         let mut sum_child_builder = sum_node_builder.get_child_builder();
    ///         let child_sum = process_node(value_child, &mut sum_child_builder);
    ///         sum += child_sum;
    ///         sum_child_builder.finish(child_sum);
    ///     }
    ///     sum
    /// }
    /// 
    /// let sum_tree = IronedTree::new(0., |node_builder| {
    ///     let sum = process_node(value_tree.root(), node_builder);
    ///     // TODO
    /// });
    /// 
    /// assert_eq!(*sum_tree.root().val(), 1.2+3.4+5.6+7.8);
    /// ```
    #[inline]
    pub fn get_tree_builder(&mut self) -> NodeBuilder<T> {
        // NodeBuilder's invariants (see comments at structure definition of NodeBuilder):
        // Invariant 1 is satisfied because the new NodeBuilder's subtree_size is 1,
        // so there are no elements with those indices.
        // Invariant 2 is satisfied, as child.index is set to self.data.len()
        let new_root_index = self.data.len();
        NodeBuilder {
            forest: self,
            index: new_root_index,
            subtree_size: NonZeroUsize::new(1).unwrap(),
            parent_subtree_size: None,
        }
    }

    /// Returns an iterator that iterates over (a [`NodeRef`] to) all the trees in this forest.
    #[inline(always)]
    pub fn iter_trees(&self) -> NodeIter<T> {
        NodeIter {
            remaining_nodes: &self.data,
        }
    }

    /// Returns an iterator that iterates over [`NodeRefMut`]s to all the trees in this forest.
    /// With this iterator you can change values of nodes in the tree (see [`NodeRefMut::val_mut`]),
    /// but you can't change the structure of the tree.
    #[inline(always)]
    pub fn iter_trees_mut(&mut self) -> NodeIterMut<T> {
        NodeIterMut {
            remaining_nodes: &mut self.data[..],
        }
    }

    /// Drain the trees in this forest.
    /// This function returns an iterator over the values of the tree, moving them out of this forest.
    /// Afterwards, the forest will be empty.
    /// 
    /// **WARNING:** if the [`NodeListDrain`] returned by this function is leaked (i.e. through [`std::mem::forget`])
    /// without iterating over all the values in it, then the values of the nodes that were not iterated over
    /// will also be leaked. Leaking is considered "safe" in Rust, so this function is safe,
    /// but you still probably want to avoid doing that.
    #[inline(always)]
    pub fn drain_trees(&mut self) -> NodeListDrain<'_, T> {
        // first, get the current length of the data vector.
        let old_len = self.data.len();
        unsafe {
            // Now we set the length to 0.
            // If we would stop here, this would leak all the values in the vector.
            // We don't have to modify `self.last_added_root_node_index` though
            // because its value is irrelevant when self.data.len() is 0.
            self.data.set_len(0);

            // Now we reconstruct a slice to the original contents of the vector.
            // This slice is pointing entirely to memory that is currently owned by the `Vec`
            // (i.e. it's within its capacity), but it's entirely out of bounds of the `Vec`,
            // so the `Vec` won't allow access to values inside this slice.
            // The `Vec` also won't drop those values.
            let mut_slice = std::slice::from_raw_parts_mut(self.data.as_mut_ptr(), old_len);

            // Finally we create a NodeListDrain<T> from this slice.
            // This NodeListDrain will read all the data out of the slice as the user
            // iterates over it, and when the NodeListDrain gets dropped,
            // it drops whatever data wasn't iterated over yet.
            // NOTE: NodeListDrain mutably borrows this IronedForest, so no changes
            // to the vector can happen while the NodeListDrain exists.
            NodeListDrain {
                remaining_nodes: mut_slice,
            }
        }
    }

    #[inline(always)]
    pub fn get(&self, index: usize) -> Option<NodeRef<T>> {
        if index < self.data.len() {
            Some(unsafe { self.get_unchecked(index) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, index: usize) -> Option<NodeRefMut<T>> {
        if index < self.data.len() {
            Some(unsafe { self.get_unchecked_mut(index) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub unsafe fn get_unchecked(&self, index: usize) -> NodeRef<T> {
        let subtree_size = self.data.get_unchecked(index).subtree_size.get();
        NodeRef {
            slice: self.data.get_unchecked(index..(index+subtree_size))
        }
    }

    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, index: usize) -> NodeRefMut<T> {
        let subtree_size = self.data.get_unchecked(index).subtree_size.get();
        NodeRefMut {
            slice: self.data.get_unchecked_mut(index..(index+subtree_size))
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        self.data.clear()
    }

    /// Iterate over all the values in all the nodes of all the trees in this forest, in pre-order order.
    #[inline(always)]
    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<T>>, impl FnMut(&'a NodeData<T>) -> &'a T>
    {
        self.data.iter().map(|node_data| &node_data.val)
    }

    /// Iterate mutably over all the values in all the nodes of all the trees in this forest, in pre-order order.
    #[inline(always)]
    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::slice::IterMut<'a, NodeData<T>>,
        impl FnMut(&'a mut NodeData<T>) -> &'a mut T,
    > {
        self.data.iter_mut().map(|node_data| &mut node_data.val)
    }

    /// Drain all the values in all the nodes of all the trees in this forest, in pre-order order.
    /// 
    /// **WARNING:** Leaking the returned iterator without iterating over all of its values will leak the
    /// values that were not iterated over.
    #[inline(always)]
    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.data.drain(..).map(|node_data| node_data.val)
    }

    /// Returns a read-only view over the raw data stored internally by this `IronedForest`.
    /// This is not really recommended to be used except for very advanced use cases.
    #[inline(always)]
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        &self.data
    }

    /// Returns how many nodes are currently in all the trees in this forest in O(1) time.
    #[inline(always)]
    pub fn tot_num_nodes(&self) -> usize {
        self.data.len()
    }
}

/// `NodeData<T>` is the data that an [`IronedForest`] or [`IronedTree`] internally stores per node:
/// the data `T` and a `usize` indicating the number of nodes in the subtree that has this node as root.
///
/// This type is not really intended to be used directly if you're a user of this library,
/// but it is nevertheless exposed if there is a reason you want to access it
/// (see e.g. [`IronedForest::raw_data`] and [`IronedTree::raw_data`])
pub struct NodeData<T> {
    val: T,
    subtree_size: NonZeroUsize,
}

impl<T> NodeData<T> {
    /// The value of the node.
    #[inline(always)]
    pub fn val(&self) -> &T {
        &self.val
    }

    /// The number of nodes in the subtree that has this node as root (i.e. this node and all its descendants).
    #[inline(always)]
    pub fn subtree_size(&self) -> NonZeroUsize {
        self.subtree_size
    }
}

/// `NodeBuilder` is a struct that lets you add children to a node that is currently being added
/// to an [`IronedTree`] or an [`IronedForest`].
/// 
/// See [`IronedTree::new`], [`IronedForest::build_tree`], [`IronedForest::get_tree_builder`], etc.
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
#[derive(destructure)]
pub struct NodeBuilder<'a, T> {
    forest: &'a mut IronedForest<T>,
    index: usize,
    subtree_size: NonZeroUsize,
    parent_subtree_size: Option<&'a mut NonZeroUsize>,
}

impl<'a, T> Drop for NodeBuilder<'a, T> {
    fn drop(&mut self) {
        unsafe {
            let data = &mut self.forest.data;

            // Drop the elements in the Vec on indices [index+1 .. index+subtree_size]
            // These are initialized, valid, and within the capacity of the Vec due to invariant 1,
            // but they are outside the len of the Vec so we can drop the data.
            //
            // Also, if this node has a parent, then we must make sure that the parent NodeBuilder won't also drop these nodes.
            // Luckily, this is the case, because self.index = parent.index+parent.subtree_size due to invariant 2,
            // so the parent's slice does *not* contain the nodes that we're about to drop due to the parent's invariant 1.
            for i in 1..self.subtree_size.get() {
                // Calculate where to read the NodeData to drop.
                // This is safe since self.index+i < data.capacity < isize::MAX
                let ptr = data.as_mut_ptr().add(self.index+i);
                let node_data : NodeData<T> = std::ptr::read(ptr);
                drop(node_data);
            }
        }
    }
}

impl<'a, T> NodeBuilder<'a, T> {
    /// test
    #[inline]
    pub fn build_child<R>(
        &mut self,
        root_val: T,
        child_builder_cb: impl FnOnce(&mut NodeBuilder<T>) -> R,
    ) -> R {
        let mut builder = self.get_child_builder();
        let ret = child_builder_cb(&mut builder);
        builder.finish(root_val);
        ret
    }

    #[inline]
    pub fn add_child(&mut self, val: T) {
        self.get_child_builder().finish(val);
    }

    #[inline]
    pub fn get_child_builder<'b>(&'b mut self) -> NodeBuilder<'b, T> {
        // Invariant 1 is satisfied because the child's NodeBuilder's subtree_size is 1,
        // so there are no elements with those indices.
        // Invariant 2 is satisfied, as child.index is set to self.index + self.subtree_size
        NodeBuilder {
            forest: &mut self.forest,
            index: self.index + self.subtree_size.get(),
            subtree_size: NonZeroUsize::new(1).unwrap(),
            parent_subtree_size: Some(&mut self.subtree_size),
        }
    }

    #[inline]
    pub fn finish(self, val: T) -> NodeRefMut<'a,T> {
        unsafe {
            // Destructure self, preventing it from being dropped.
            // We do this as the very first thing so that if at any point during this function there is a panic,
            // we can be sure that there won't be a double drop (worst case scenario there's a leak, which is safe).
            let (forest, index, subtree_size, mut parent_subtree_size_ref_mut) = self.destructure();

            let data = &mut forest.data;
            let data_len = data.len();

            // Check (part of) invariant 1
            debug_assert!(index >= data_len);

            // Make sure data can hold at least self.index + self.subtree_size elements
            // I'd like to just call data.reserve(self.index + self.subtree_size.get() - data_len) and be done with it.
            // Unfortunately, if there's a reallocation, the data between data.capacity() and data.len() is not
            // guaranteed to be copied over (under the current implementation at the time of writing it is,
            // but it's not guaranteed to be).
            //
            // So what we do instead is this:
            //
            // First, check if the current capacity is already enough. If so, do nothing.
            let needed_capacity = index + subtree_size.get();
            let cur_capacity = data.capacity();
            if needed_capacity > cur_capacity {
                // In this branch the current capacity is not enough.

                // We use set_len() to guarantee that if there is a reallocation,
                // the data that we've been writing gets copied over.
                data.set_len(cur_capacity);
                data.reserve(needed_capacity - data_len);
                data.set_len(data_len);

                // TODO: rework using from_raw_parts
            }
            
            // Calculate where to write the data.
            // This is safe since self.index < data.capacity < isize::MAX
            let ptr = data.as_mut_ptr().add(index);

            // Write NodeData to the forest at calculated location
            // This is outside the len, but inside the capacity
            std::ptr::write(ptr, NodeData {
                val,
                subtree_size
            });

            if let Some(ref mut parent_subtree_size) = parent_subtree_size_ref_mut {
                // There is a parent, so we should update its subtree_size to include this Node and descendants.
                // Since this node has self.subtree_size descendants (including itself), this means adding
                // self.subtree_size to parent.subtree_size.
                std::mem::replace(*parent_subtree_size, NonZeroUsize::new_unchecked(parent_subtree_size.get() + subtree_size.get()));

                // We need to prove that the parent's invariants are not violated here.
                //
                // Let's give things some shorter names to be able to talk about them more easily.
                //   SI = self.index
                //   SS = self.subtree_size
                //   PI = parent.index
                //   POS = parent's old subtree_size
                //   PNS = parent's new subtree_size = POS+SS
                //
                // The parent's invariants require that the nodes at indices [PI+1..PI+PNS]
                // are valid and initialized and within the capacity of the Vec.
                //
                // Due to its invariant 1, [PI+1..PI+POS] were already initialized,
                // so we only need to prove that [PI+POS..PI+PNS] are initialized.
                //
                // Due to our invariant 2, SI == PI+POS, and because PNS=POS+SS,
                // what we really need to prove is that [SI..SI+SS] are initialized.
                //
                // Due to our invariant 1, [SI+1..SI+SS] are initialized,
                // and the node at index SI was initialized above using ptr::write.
                //
                // The capacity was also set to (at least) SI+SS = PI+POS+SS = PI+PNS above,
                // through data.reserve(...), so the capacity is also ok.
            } else {
                // When this node has no parent, we're done initializing all nodes and
                // can update the len of the forest's data vector.
                
                // The current len should be equal to self.index (see invariant 2)
                debug_assert_eq!(index, data_len);

                // We now add self.subtree_size to that length.
                //
                // Safety requirements of set_len():
                //
                // 1. new_len must be less than or equal to capacity().
                // We called data.reserve() above requesting precisely this many elements of capacity.
                //
                // 2. The elements at old_len..new_len must be initialized.
                // There's no data between old_len and self.index (see above),
                // the data at index self.index was initialized earlier in this function,
                // and the data at indices [self.index+1..self.index+self.subtree_size]
                // are initialized due to invariant 1.
                data.set_len(index + subtree_size.get());
            }
            
            NodeRefMut {
                slice: forest.data.get_unchecked_mut(index .. (index+subtree_size.get()))
            }
        }
    }
}

/// test
#[derive(Copy)]
pub struct NodeIter<'t, T> {
    remaining_nodes: &'t [NodeData<T>], // contains (only) the nodes in the iterator and all their descendants
}

impl<'t, T> Clone for NodeIter<'t, T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self {
            remaining_nodes: self.remaining_nodes,
        }
    }
}

impl<'t, T> NodeIter<'t, T> {
    #[inline(always)]
    pub fn remaining_subtrees_size(&self) -> usize {
        self.remaining_nodes.len()
    }
}

impl<'t, T> Iterator for NodeIter<'t, T> {
    type Item = NodeRef<'t, T>;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes.get(0).map(|cur_node| {
            NodeRef {
                slice: unsafe { slice_split_off_first_n_unchecked(&mut self.remaining_nodes, cur_node.subtree_size.get()) }
            }
        })
    }
}

/// test
pub struct NodeRef<'t, T> {
    slice: &'t [NodeData<T>], // contains (only) the current node and all its descendants
}

// Not using #[derive(Copy)] because it adds the T:Copy bound, which is unnecessary
impl<'t,T> Copy for NodeRef<'t,T> {}

// Not using #[derive(Clone)] because it adds the T:Clone bound, which is unnecessary
impl<'t,T> Clone for NodeRef<'t,T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        *self
    }
}

impl<'t, T> NodeRef<'t, T> {
    #[inline(always)]
    pub fn children(&self) -> NodeIter<'t, T> {
        let (_, remaining_nodes) = unsafe { slice_split_first_unchecked(self.slice) };
        NodeIter { remaining_nodes }
    }

    #[inline(always)]
    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }

    #[inline(always)]
    pub fn num_descendants_incl_self(&self) -> usize {
        self.slice.len()
    }

    #[inline(always)]
    pub fn num_descendants_excl_self(&self) -> usize {
        self.slice.len() - 1
    }
}

pub struct NodeIterMut<'t, T> {
    remaining_nodes: &'t mut [NodeData<T>], // contains (only) the nodes in the iterator and all their descendants
}

impl<'t, T> Iterator for NodeIterMut<'t, T> {
    type Item = NodeRefMut<'t, T>;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cur_node) = self.remaining_nodes.get(0) {
            let cur_node_subtree_size = cur_node.subtree_size.get();
            Some(NodeRefMut {
                slice: unsafe { slice_split_off_first_n_unchecked_mut(&mut self.remaining_nodes, cur_node_subtree_size) }
            })
        } else {
            None
        }
    }
}

impl<'t, T> NodeIterMut<'t, T> {
    #[inline(always)]
    pub fn remaining_subtrees_size(&self) -> usize {
        self.remaining_nodes.len()
    }
}

pub struct NodeRefMut<'t, T> {
    slice: &'t mut [NodeData<T>], // contains (only) the current node and all its descendants
}

impl<'t, T> NodeRefMut<'t, T> {
    #[inline(always)]
    pub fn into_children(self) -> NodeIterMut<'t, T> {
        let (_, remaining_nodes) = unsafe { slice_split_first_unchecked_mut(self.slice) };
        NodeIterMut { remaining_nodes }
    }

    #[inline(always)]
    pub fn children(&mut self) -> NodeIterMut<T> {
        let (_, remaining_nodes) = unsafe { slice_split_first_unchecked_mut(self.slice) };
        NodeIterMut { remaining_nodes }
    }

    #[inline(always)]
    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }

    #[inline(always)]
    pub fn val_mut(&mut self) -> &mut T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &mut self.slice.get_unchecked_mut(0).val }
    }

    #[inline(always)]
    pub fn num_descendants_incl_self(&self) -> usize {
        self.slice.len()
    }

    #[inline(always)]
    pub fn num_descendants_excl_self(&self) -> usize {
        self.slice.len() - 1
    }
}

impl<'t,T> From<NodeRefMut<'t,T>> for NodeRef<'t,T> {
    fn from(val: NodeRefMut<'t,T>) -> Self {
        NodeRef {
            slice: val.slice
        }
    }
}

pub struct NodeListDrain<'t, T> {
    // `remaining_nodes` is a slice containing (only) the remaining nodes in the iterator and all their descendants.
    // Normally slices don't own data, but not in this case.
    // The data is actually owned by the Vec that this NodeListDrain borrows, but it's out of the bounds of that Vec (but still inside its capacity).
    // Therefore the NodeListDrain can pretend like it owns the data in this slice, it can drop them in drop(),
    // and it can move out values using ptr::read (as long as it makes sure to update the slice to prevent a double drop)
    remaining_nodes: &'t mut [NodeData<T>],
}

impl<'t, T> Drop for NodeListDrain<'t, T> {
    fn drop(&mut self) {
        // read out all values in the slice and drop them
        for node in self.remaining_nodes.iter_mut() {
            unsafe {
                let value: NodeData<T> = std::ptr::read(node);
                std::mem::drop(value); // not strictly needed
            }
        }
    }
}

impl<'t, T> Iterator for NodeListDrain<'t, T> {
    type Item = NodeDrain<'t, T>;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(cur_node) = self.remaining_nodes.get(0) {
            let cur_node_subtree_size = cur_node.subtree_size.get();
            Some(NodeDrain {
                slice: unsafe { slice_split_off_first_n_unchecked_mut(&mut self.remaining_nodes, cur_node_subtree_size) }
            })
        } else {
            None
        }
    }
}

impl<'t, T> NodeListDrain<'t, T> {
    #[inline(always)]
    pub fn remaining_subtrees_size(&self) -> usize {
        self.remaining_nodes.len()
    }
}

pub struct NodeDrain<'t, T> {
    // `remaining_nodes` is a slice containing (only) the current node (i.e., the first node in the slice) and all its descendants.
    // Normally slices don't own data, but not in this case.
    // The data is actually owned by the Vec that this NodeDrain borrows, but it's out of the bounds of that Vec (but still inside its capacity).
    // Therefore the NodeDrain can pretend like it owns the data in this slice, it can drop them in drop(),
    // and it can move out values using ptr::read (as long as it makes sure to update the slice to prevent a double drop)
    slice: &'t mut [NodeData<T>],
}

impl<'t, T> Drop for NodeDrain<'t, T> {
    #[inline]
    fn drop(&mut self) {
        // read out all values in the slice and drop them
        for node in self.slice.iter_mut() {
            unsafe {
                let value: NodeData<T> = std::ptr::read(node);
                std::mem::drop(value); // not strictly needed
            }
        }
    }
}

impl<'t, T> NodeDrain<'t, T> {
    #[inline(always)]
    pub fn into_val_and_children(mut self) -> (T, NodeListDrain<'t, T>) {
        // move the slice out of self, so it won't drop the data anymore
        let slice = std::mem::replace(&mut self.slice, &mut []);

        // split off the first element
        let (node_data_ref, remaining_nodes) = slice.split_first_mut().unwrap();

        unsafe {
            // read the NodeData out of the ref we have to it
            let node_data: NodeData<T> = std::ptr::read(node_data_ref);

            // Return the value (the user will drop it)
            // and the remaining slice as a NodeListDrain, which now owns the values in that slice (and will drop them)
            (node_data.val, NodeListDrain { remaining_nodes })
        }
    }

    #[inline(always)]
    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }

    #[inline(always)]
    pub fn val_mut(&mut self) -> &mut T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &mut self.slice.get_unchecked_mut(0).val }
    }

    #[inline(always)]
    pub fn num_descendants_incl_self(&self) -> usize {
        self.slice.len()
    }

    #[inline(always)]
    pub fn num_descendants_excl_self(&self) -> usize {
        self.slice.len() - 1
    }
}
