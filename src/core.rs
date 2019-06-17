// TODO: NodeRefMut to NodeRef

// core.rs contains all the unsafe code.
// It should be kept as small as possible.
// No bugs outside of core.rs should lead to memory unsafety.
use std::convert::{From, TryFrom};
use std::iter::Iterator;
use std::num::NonZeroUsize;

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
    last_added_root_node_index: usize, // used to update `next_sibling_offset` of the last root node when adding a tree. Only valid when data.len() > 0
}

impl<T> IronedForest<T> {
    /// Create a new [`IronedForest`].
    pub fn new() -> IronedForest<T> {
        IronedForest {
            data: Vec::new(),
            last_added_root_node_index: 0,
        }
    }

    /// Create a new [`IronedForest`] with the specified capacity for the inner `Vec` which stores the nodes (see [`Vec::with_capacity`]).
    pub fn with_capacity(capacity: usize) -> IronedForest<T> {
        IronedForest {
            data: Vec::with_capacity(capacity),
            last_added_root_node_index: 0,
        }
    }

    /// Build a tree and add it to the forest.
    /// 
    /// The parameter `root_initial_val` is the value that the root of the tree will have (unless it's modified while building the tree,
    /// through [`NodeBuilder::val_mut`]).
    /// 
    /// The parameter `node_builder_cb` is a callback function that is called exactly once. It is passed a [`NodeBuilder`] that can be
    /// used to add nodes to the root node. The value returned by `node_builder_cb` becomes the return value of this function.
    /// 
    /// For complex use cases where callbacks can get in the way, [`get_tree_builder`](`IronedForest::get_tree_builder`) may be more ergonomic.
    pub fn build_tree<R>(
        &mut self,
        root_initial_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.get_tree_builder(root_initial_val))
    }

    /// Add a tree with only a single node to the forest. The parameter `val` is the value of that single node.
    pub fn add_single_node_tree(&mut self, val: T) {
        self.get_tree_builder(val);
    }

    /// Get a [`NodeBuilder`] that can be used to build a tree that will be added to this forest.
    /// 
    /// The parameter `root_initial_val` is the value that the root node will have, unless it is modified later
    /// through [`NodeBuilder::val_mut`]).
    /// 
    /// The [`NodeBuilder`] borrows the forest mutably, so you can't do anything with the forest until you're
    /// done building the tree.
    /// 
    /// For simple use cases, using [`build_tree`](`IronedForest::build_tree`) is more ergonomic.
    pub fn get_tree_builder(&mut self, root_initial_val: T) -> NodeBuilder<T> {
        let new_root_node_index = self.data.len();
        self.data.push(NodeData {
            val: root_initial_val,
            next_sibling_offset: None,
        });

        // update next_sibling_offset of the last added root node (if any. There isn't any if and only if new_root_node_index == 0)
        if new_root_node_index > 0 {
            debug_assert!(self.last_added_root_node_index < new_root_node_index);
            let offset = new_root_node_index - self.last_added_root_node_index;
            debug_assert!(offset > 0);
            unsafe {
                self.data
                    .get_unchecked_mut(self.last_added_root_node_index)
                    .next_sibling_offset = Some(NonZeroUsize::new_unchecked(offset));
            }
        }
        self.last_added_root_node_index = new_root_node_index;

        NodeBuilder {
            store: self,
            index: new_root_node_index,
            last_added_child_index: None,
        }
    }

    /// Returns an iterator that iterates over (a [`NodeRef`] to) all the trees in this forest.
    pub fn iter_trees(&self) -> NodeIter<T> {
        NodeIter {
            remaining_nodes: &self.data,
        }
    }

    /// Returns an iterator that iterates over [`NodeRefMut`]s to all the trees in this forest.
    /// With this iterator you can change values of nodes in the tree (see [`NodeRefMut::val_mut`]),
    /// but you can't change the structure of the tree.
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

    /// Iterate over all the values in all the nodes of all the trees in this forest, in pre-order order.
    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<T>>, impl FnMut(&'a NodeData<T>) -> &'a T>
    {
        self.data.iter().map(|node_data| &node_data.val)
    }

    /// Iterate mutably over all the values in all the nodes of all the trees in this forest, in pre-order order.
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
    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.data.drain(..).map(|node_data| node_data.val)
    }

    /// Returns a read-only view over the raw data stored internally by this `IronedForest`.
    /// This is not really recommended to be used except for very advanced use cases.
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        &self.data
    }

    /// Returns how many nodes are currently in all the trees in this forest in O(1) time.
    pub fn tot_num_nodes(&self) -> usize {
        self.data.len()
    }
}

/// `NodeData<T>` is the data that an [`IronedForest`] or [`IronedTree`] internally stores per node:
/// the data `T` and a `usize` pointing to the next sibling of this node (0 if there is no next sibling).
///
/// This type is not really intended to be used directly if you're a user of this library,
/// but it is nevertheless exposed if there is a reason you want to access it
/// (see e.g. [`IronedForest::raw_data`] and [`IronedTree::raw_data`])
pub struct NodeData<T> {
    val: T,
    next_sibling_offset: Option<NonZeroUsize>, // Difference between the index of the next sibling and the index of the current node. None if there is no next sibling.
}

impl<T> NodeData<T> {
    /// The value of the node.
    pub fn val(&self) -> &T {
        &self.val
    }

    /// The offset (in the `Vec` that this [`IronedForest`] or [`IronedTree`] stores) to the next sibling of the current node (or `None` if there is no next sibling).
    pub fn next_sibling_offset(&self) -> Option<NonZeroUsize> {
        self.next_sibling_offset
    }
}

/// `NodeBuilder` is a struct that lets you add children to a node that is currently being added
/// to an [`IronedTree`] or an [`IronedForest`].
/// 
/// See [`IronedTree::new`], [`IronedForest::build_tree`], [`IronedForest::get_tree_builder`], etc.
pub struct NodeBuilder<'a, T> {
    store: &'a mut IronedForest<T>,
    index: usize,                          // index of the node that we are constructing
    last_added_child_index: Option<usize>, // to update next_sibling_offset
}

impl<'a, T> NodeBuilder<'a, T> {
    /// Read the value of the node that is currently being built.
    pub fn val(&self) -> &T {
        unsafe { &self.store.data.get_unchecked(self.index).val }
    }

    /// Mutably access the value of the node that is currently being built.
    /// 
    /// This is useful when at the you don't yet know what the value of a node will be
    /// before adding all the children to it.
    /// 
    /// Example:
    /// ```
    /// use tree_iron::{IronedTree, NodeRef, NodeBuilder};
    /// 
    /// // Assume you already have some kind of tree with floating point values, like this:
    /// let value_tree = IronedTree::new(1.2, |mut node| {
    ///     node.build_child(3.4, |mut node| {
    ///         node.add_child(5.6);
    ///     });
    ///     node.add_child(7.8);
    /// });
    /// 
    /// // Build a tree from the previous tree,
    /// // where the value of a node is the sum of the values
    /// // of all the values of all the nodes below it (including itself).
    /// // Returns 
    /// fn process_node(value_node: NodeRef<f64>, sum_node_builder: &mut NodeBuilder<f64>) {
    ///     let mut sum = *value_node.val();
    ///     for value_child in value_node.children() {
    ///         let mut sum_child_builder = sum_node_builder.get_child_builder(0.);
    ///         process_node(value_child, &mut sum_child_builder);
    ///         sum += sum_child_builder.val();
    ///     }
    ///     *sum_node_builder.val_mut() = sum;
    /// }
    /// 
    /// let sum_tree = IronedTree::new(0., |mut node_builder| {
    ///     let sum = process_node(value_tree.root(), &mut node_builder);
    /// });
    /// 
    /// assert_eq!(*sum_tree.root().val(), 1.2+3.4+5.6+7.8);
    /// ```
    pub fn val_mut(&mut self) -> &mut T {
        unsafe { &mut self.store.data.get_unchecked_mut(self.index).val }
    }

    /// test
    pub fn build_child<R>(
        &mut self,
        initial_val: T,
        child_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        child_builder_cb(self.get_child_builder(initial_val))
    }

    pub fn add_child(&mut self, initial_val: T) {
        self.get_child_builder(initial_val);
    }

    pub fn get_child_builder<'b>(&'b mut self, initial_val: T) -> NodeBuilder<'b, T> {
        let child_node_index = self.store.data.len();
        self.store.data.push(NodeData {
            val: initial_val,
            next_sibling_offset: None,
        });

        // update next_sibling_offset of the last added node (if any)
        if let Some(last_added_child_index) = self.last_added_child_index {
            debug_assert!(last_added_child_index < child_node_index);
            let offset = child_node_index - last_added_child_index;
            debug_assert!(offset > 0);
            unsafe {
                self.store
                    .data
                    .get_unchecked_mut(last_added_child_index)
                    .next_sibling_offset = Some(NonZeroUsize::new_unchecked(offset));
            }
        }
        self.last_added_child_index = Some(child_node_index);

        NodeBuilder {
            store: self.store,
            index: child_node_index,
            last_added_child_index: None,
        }
    }
}

/// test
#[derive(Copy)]
pub struct NodeIter<'t, T> {
    remaining_nodes: &'t [NodeData<T>], // contains (only) the nodes in the iterator and all their descendants
}

impl<'t, T> Clone for NodeIter<'t, T> {
    fn clone(&self) -> Self {
        Self {
            remaining_nodes: self.remaining_nodes,
        }
    }
}

impl<'t, T> NodeIter<'t, T> {
    pub fn remaining_subtrees_size(&self) -> usize {
        self.remaining_nodes.len()
    }
}

impl<'t, T> Iterator for NodeIter<'t, T> {
    type Item = NodeRef<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes.get(0).map(|cur_node| {
            if let Some(next_sibling_offset) = cur_node.next_sibling_offset {
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &[]);
                let (cur_node_slice, next_nodes_slice) =
                    remaining_nodes.split_at(next_sibling_offset.get());
                self.remaining_nodes = next_nodes_slice;
                NodeRef {
                    slice: cur_node_slice,
                }
            } else {
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &[]);
                NodeRef {
                    slice: remaining_nodes,
                }
            }
        })
    }
}

/// test
#[derive(Clone, Copy)]
pub struct NodeRef<'t, T> {
    slice: &'t [NodeData<T>], // contains (only) the current node and all its descendants
}

impl<'t, T> NodeRef<'t, T> {
    pub fn children(&self) -> NodeIter<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first().unwrap();
        NodeIter { remaining_nodes }
    }

    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }

    pub fn num_descendants_incl_self(&self) -> usize {
        self.slice.len()
    }

    pub fn num_descendants_excl_self(&self) -> usize {
        self.slice.len() - 1
    }
}

pub struct NodeIterMut<'t, T> {
    remaining_nodes: &'t mut [NodeData<T>], // contains (only) the nodes in the iterator and all their descendants
}

impl<'t, T> Iterator for NodeIterMut<'t, T> {
    type Item = NodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| cur_node.next_sibling_offset)
            .map(|maybe_next_sibling_offset| {
                if let Some(next_sibling_offset) = maybe_next_sibling_offset {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    let (cur_node_slice, next_nodes_slice) =
                        remaining_nodes.split_at_mut(next_sibling_offset.get());
                    self.remaining_nodes = next_nodes_slice;
                    NodeRefMut {
                        slice: cur_node_slice,
                    }
                } else {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    NodeRefMut {
                        slice: remaining_nodes,
                    }
                }
            })
    }
}

impl<'t, T> NodeIterMut<'t, T> {
    pub fn remaining_subtrees_size(&self) -> usize {
        self.remaining_nodes.len()
    }
}

pub struct NodeRefMut<'t, T> {
    slice: &'t mut [NodeData<T>], // contains (only) the current node and all its descendants
}

impl<'t, T> NodeRefMut<'t, T> {
    pub fn into_children(self) -> NodeIterMut<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first_mut().unwrap();
        NodeIterMut { remaining_nodes }
    }

    pub fn children(&mut self) -> NodeIterMut<T> {
        let (_, remaining_nodes) = self.slice.split_first_mut().unwrap();
        NodeIterMut { remaining_nodes }
    }

    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }

    pub fn val_mut(&mut self) -> &mut T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &mut self.slice.get_unchecked_mut(0).val }
    }

    pub fn num_descendants_incl_self(&self) -> usize {
        self.slice.len()
    }

    pub fn num_descendants_excl_self(&self) -> usize {
        self.slice.len() - 1
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
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| cur_node.next_sibling_offset)
            .map(|maybe_next_sibling_offset| {
                // move the slice out of self, so it won't drop the data anymore
                let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);

                if let Some(next_sibling_offset) = maybe_next_sibling_offset {
                    // split off the first node and its descendants
                    let (cur_node_slice, next_nodes_slice) =
                        remaining_nodes.split_at_mut(next_sibling_offset.get());

                    // update self.remaining_nodes so we drop them again and for future calls to next()
                    self.remaining_nodes = next_nodes_slice;

                    NodeDrain {
                        slice: cur_node_slice,
                    }
                } else {
                    NodeDrain {
                        slice: remaining_nodes,
                    }
                }
            })
    }
}

impl<'t, T> NodeListDrain<'t, T> {
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
    pub fn into_val_and_children(mut self) -> (T, NodeListDrain<'t, T>) {
        // move the slice out of self, so it won't drop the data anymore
        let slice = std::mem::replace(&mut self.slice, &mut []);

        // split off the first element
        let (node_data_ref, remaining_nodes) = slice.split_first_mut().unwrap();

        unsafe {
            // read the NodeData out of the ref we have to it
            let node_data: NodeData<T> = std::ptr::read(node_data_ref);

            // Return the value (the user will drop it)
            // and the remaining slice as a NodeListDrain, who now owns the values in that slice (and will drop them)
            (node_data.val, NodeListDrain { remaining_nodes })
        }
    }

    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }

    pub fn val_mut(&mut self) -> &mut T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &mut self.slice.get_unchecked_mut(0).val }
    }

    pub fn num_descendants_incl_self(&self) -> usize {
        self.slice.len()
    }

    pub fn num_descendants_excl_self(&self) -> usize {
        self.slice.len() - 1
    }
}

/// test
pub struct IronedTree<T> {
    forest: IronedForest<T>,
}

impl<T> IronedTree<T> {
    pub fn new(root_val: T, node_builder_cb: impl FnOnce(NodeBuilder<T>)) -> IronedTree<T> {
        IronedTree::new_with_return_val(root_val, node_builder_cb).0
    }

    pub fn new_with_return_val<R>(
        root_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> (IronedTree<T>, R) {
        let mut forest = IronedForest::new();
        let ret = forest.build_tree(root_val, node_builder_cb);
        (IronedTree { forest }, ret)
    }

    pub fn new_with_capacity(
        root_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>),
        capacity: usize,
    ) -> IronedTree<T> {
        IronedTree::new_with_capacity_and_return_val(root_val, node_builder_cb, capacity).0
    }

    pub fn new_with_capacity_and_return_val<R>(
        root_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
        capacity: usize,
    ) -> (IronedTree<T>, R) {
        let mut forest = IronedForest::with_capacity(capacity);
        let ret = forest.build_tree(root_val, node_builder_cb);
        (IronedTree { forest }, ret)
    }

    pub fn root(&self) -> NodeRef<T> {
        self.forest.iter_trees().next().unwrap()
    }

    pub fn root_mut(&mut self) -> NodeRefMut<T> {
        self.forest.iter_trees_mut().next().unwrap()
    }

    pub fn drain_root(&mut self) -> NodeDrain<T> {
        self.forest.drain_trees().next().unwrap()
    }

    pub fn iter_flattened<'a>(
        &'a self,
    ) -> std::iter::Map<std::slice::Iter<'a, NodeData<T>>, impl FnMut(&'a NodeData<T>) -> &'a T>
    {
        self.forest.iter_flattened()
    }

    pub fn iter_flattened_mut<'a>(
        &'a mut self,
    ) -> std::iter::Map<
        std::slice::IterMut<'a, NodeData<T>>,
        impl FnMut(&'a mut NodeData<T>) -> &'a mut T,
    > {
        self.forest.iter_flattened_mut()
    }

    pub fn drain_flattened(
        &mut self,
    ) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.forest.drain_flattened()
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        self.forest.raw_data()
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.forest.tot_num_nodes()
    }
}

impl<T> TryFrom<IronedForest<T>> for IronedTree<T> {
    type Error = ();
    fn try_from(forest: IronedForest<T>) -> Result<Self, Self::Error> {
        if forest.iter_trees().count() == 1 {
            Ok(IronedTree { forest })
        } else {
            Err(())
        }
    }
}

impl<T> From<IronedTree<T>> for IronedForest<T> {
    fn from(tree: IronedTree<T>) -> Self {
        IronedForest {
            data: tree.forest.data,
            last_added_root_node_index: 0,
        }
    }
}
