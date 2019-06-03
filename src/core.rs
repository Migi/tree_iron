/// Core contains all the unsafe code.
/// It should be kept as small as possible.
use std::iter::Iterator;
use std::mem::ManuallyDrop;
use std::num::NonZeroUsize;

/// test
pub struct TreeStore<T> {
    data: Vec<ManuallyDrop<NodeData<T>>>, // all data is dropped in drop(), only drain() prevents this
}

impl<T> Drop for TreeStore<T> {
    fn drop(&mut self) {
        for node in self.data.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<T> TreeStore<T> {
    pub fn new() -> TreeStore<T> {
        TreeStore {
            data: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> TreeStore<T> {
        TreeStore {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn build_tree<R>(
        &mut self,
        initial_val: T,
        node_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        node_builder_cb(self.build_child(initial_val))
    }

    pub fn add_tree<'a>(
        &'a mut self,
        initial_val: T
    ) -> NodeBuilder<'a,T> {
        let mut node_builder = NodeBuilder {
            store: self,
            index: 0, // this index is wrong, but `index` is not used in build_child()
            last_added_child_index: None
        };
        node_builder.build_child(initial_val, node_builder_cb)
    }

    pub fn iter_trees(&self) -> NodeIter<T> {
        NodeIter {
            remaining_nodes: &self.data,
        }
    }

    pub fn iter_trees_mut(&mut self) -> NodeIterMut<T> {
        NodeIterMut {
            remaining_nodes: &mut self.data[..]
        }
    }

    pub fn drain_trees(mut self) -> TreeDrain<T> {
        let data = std::mem::replace(&mut self.data, Vec::new());
        TreeDrain { data, drop_from: 0 }
    }

    /// Read-only view of the raw data.
    pub fn raw_data(&self) -> &Vec<ManuallyDrop<NodeData<T>>> {
        &self.data
    }
}

pub struct NodeData<T> {
    val: T,
    next_sibling_offset: Option<NonZeroUsize> // Difference between the index of the next sibling and the index of the current node. None if there is no next sibling.
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
    store: &'a mut TreeStore<T>,
    index: usize, // index of the node that we are constructing
    last_added_child_index: Option<usize>, // to update next_sibling_offset
}

impl<'a, T> NodeBuilder<'a, T> {
    pub fn val(&self) -> &T {
        unsafe {
            &self.store.data.get_unchecked(self.index).val
        }
    }

    pub fn val_mut(&mut self) -> &mut T {
        unsafe {
            &mut self.store.data.get_unchecked_mut(self.index).val
        }
    }

    pub fn build_child<R>(
        &mut self,
        initial_val: T,
        child_builder_cb: impl FnOnce(NodeBuilder<T>) -> R,
    ) -> R {
        child_builder_cb(self.build_child(initial_val))
    }

    pub fn add_child<'a>(
        &'a mut self,
        initial_val: T
    ) -> NodeBuilder<'a,T> {
        let child_node_index = self.store.data.len();
        self.store.data.push(ManuallyDrop::new(NodeData {
            val: initial_val,
            next_sibling_offset: None,
        }));

        // update next_sibling_offset of the last added node (if any)
        if let Some(last_added_child_index) = self.last_added_child_index {
            debug_assert!(last_added_child_index < child_node_index);
            let offset = child_node_index - last_added_child_index;
            debug_assert!(offset != 0);
            unsafe {
                self.store.data.get_unchecked_mut(last_added_child_index).next_sibling_offset = Some(NonZeroUsize::new_unchecked(offset));
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
    remaining_nodes: &'t [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants
}

impl<'t, T> Clone for NodeIter<'t, T> {
    fn clone(&self) -> Self {
        Self {
            remaining_nodes: self.remaining_nodes
        }
    }
}

/// test
#[derive(Clone,Copy)]
pub struct NodeRef<'t, T> {
    slice: &'t [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants
}

impl<'t, T> Iterator for NodeIter<'t, T> {
    type Item = NodeRef<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| {
                if let Some(next_sibling_offset) = cur_node.next_sibling_offset {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &[]);
                    let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at(next_sibling_offset.get());
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

impl<'t, T> NodeRef<'t, T> {
    pub fn children(&self) -> NodeIter<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first().unwrap();
        NodeIter { remaining_nodes }
    }

    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }
}

pub struct NodeIterMut<'t, T> {
    remaining_nodes: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants
}

pub struct NodeRefMut<'t, T> {
    slice: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants
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
                    let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_sibling_offset.get());
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

impl<'t, T> NodeRefMut<'t, T> {
    pub fn children(self) -> NodeIterMut<'t, T> {
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
}

pub struct TreeDrain<T> {
    data: Vec<ManuallyDrop<NodeData<T>>>, // only data from `drop_from` to the end of the vec is dropped
    drop_from: usize,
}

pub struct NodeListDrain<'t, T> {
    remaining_nodes: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants. Drops them in drop().
}

pub struct NodeDrain<'t, T> {
    slice: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants. Drops them in drop().
}

impl<T> Drop for TreeDrain<T> {
    fn drop(&mut self) {
        for node in self.data[self.drop_from..].iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<'t, T> Drop for NodeListDrain<'t, T> {
    fn drop(&mut self) {
        for node in self.remaining_nodes.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<'t, T> Drop for NodeDrain<'t, T> {
    fn drop(&mut self) {
        for node in self.slice.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<T> TreeDrain<T> {
    pub fn drain_all(&mut self) -> NodeListDrain<T> {
        let drop_from = self.drop_from;
        self.drop_from = self.data.len();
        NodeListDrain {
            remaining_nodes: &mut self.data[drop_from..],
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
                if let Some(next_sibling_offset) = maybe_next_sibling_offset {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_sibling_offset.get());
                    self.remaining_nodes = next_nodes_slice;
                    NodeDrain {
                        slice: cur_node_slice,
                    }
                } else {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    NodeDrain {
                        slice: remaining_nodes,
                    }
                }
            })
    }
}

impl<'t, T> NodeDrain<'t, T> {
    pub fn into_val_and_children(mut self) -> (T, NodeListDrain<'t, T>) {
        let slice = std::mem::replace(&mut self.slice, &mut []);
        let (node_data_ref, remaining_nodes) = slice.split_first_mut().unwrap();
        unsafe {
            let node_data: NodeData<T> = std::ptr::read(&**node_data_ref); // TODO: replace with ManuallyDrop::take() once that is stabilized
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
}
