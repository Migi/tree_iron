/// Core contains all the unsafe code.
/// It should be kept as small as possible.
use std::iter::Iterator;
use std::num::NonZeroUsize;

/// test
pub struct IronedForest<T> {
    data: Vec<NodeData<T>>,
    last_added_root_node_index: usize, // used to update `next_sibling_offset` of the last root node when adding a tree. Only valid when data.len() > 0
}

impl<T> IronedForest<T> {
    pub fn new() -> IronedForest<T> {
        IronedForest {
            data: Vec::new(),
            last_added_root_node_index: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> IronedForest<T> {
        IronedForest {
            data: Vec::with_capacity(capacity),
            last_added_root_node_index: 0,
        }
    }

    pub fn build_tree<R>(
        &mut self,
        initial_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.add_tree(initial_val))
    }

    pub fn add_tree(&mut self, initial_val: T) -> NodeBuilder<T> {
        let new_root_node_index = self.data.len();
        self.data.push(NodeData {
            val: initial_val,
            next_sibling_offset: None,
        });

        // update next_sibling_offset of the last added root node (if any, there won't be any if and only if new_root_node_index == 0)
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

    pub fn iter_trees(&self) -> NodeIter<T> {
        NodeIter {
            remaining_nodes: &self.data,
        }
    }

    pub fn iter_trees_mut(&mut self) -> NodeIterMut<T> {
        NodeIterMut {
            remaining_nodes: &mut self.data[..],
        }
    }

    pub fn drain_trees(&mut self) -> NodeListDrain<'_, T> {
        // first, get the current length of the data vector.
        let old_len = self.data.len();
        unsafe {
            // Now we set the length to 0.
            // If we would stop here, this would leak all the memory in the vector.
            self.data.set_len(0);

            // Now we reconstruct a slice to the original contents of the vector.
            // This slice is pointing entirely to memory that is currently allocated
            // (i.e. within the capacity of the vector), but won't get dropped.
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

    pub fn iter_flattened<'a>(&'a self) -> std::iter::Map<std::slice::Iter<'a, NodeData<T>>, impl FnMut(&'a NodeData<T>) -> &'a T> {
        self.data.iter().map(|node_data| &node_data.val)
    }

    pub fn iter_flattened_mut<'a>(&'a mut self) -> std::iter::Map<std::slice::IterMut<'a, NodeData<T>>, impl FnMut(&'a mut NodeData<T>) -> &'a mut T> {
        self.data.iter_mut().map(|node_data| &mut node_data.val)
    }

    pub fn drain_flattened(&mut self) -> std::iter::Map<std::vec::Drain<NodeData<T>>, impl FnMut(NodeData<T>) -> T> {
        self.data.drain(..).map(|node_data| node_data.val)
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<NodeData<T>> {
        &self.data
    }

    pub fn tot_num_nodes(&self) -> usize {
        self.data.len()
    }
}

pub struct NodeData<T> {
    val: T,
    next_sibling_offset: Option<NonZeroUsize>, // Difference between the index of the next sibling and the index of the current node. None if there is no next sibling.
}

impl<T> NodeData<T> {
    pub fn val(&self) -> &T {
        &self.val
    }

    pub fn next_sibling_offset(&self) -> Option<NonZeroUsize> {
        self.next_sibling_offset
    }
}

/// test
pub struct NodeBuilder<'a, T> {
    store: &'a mut IronedForest<T>,
    index: usize,                          // index of the node that we are constructing
    last_added_child_index: Option<usize>, // to update next_sibling_offset
}

impl<'a, T> NodeBuilder<'a, T> {
    pub fn val(&self) -> &T {
        unsafe { &self.store.data.get_unchecked(self.index).val }
    }

    pub fn val_mut(&mut self) -> &mut T {
        unsafe { &mut self.store.data.get_unchecked_mut(self.index).val }
    }

    pub fn build_child<R>(
        &mut self,
        initial_val: T,
        child_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        child_builder_cb(self.add_child(initial_val))
    }

    pub fn add_child(&mut self, initial_val: T) -> NodeBuilder<T> {
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
