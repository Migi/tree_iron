/// Core contains all the unsafe code.
/// It should be kept as small as possible.
use std::iter::Iterator;
use std::mem::ManuallyDrop;
use std::num::NonZeroUsize;

/// test
pub struct Immutree<T> {
    data: Vec<ManuallyDrop<NodeData<T>>>, // all data is dropped in drop(), only drain() prevents this
}

impl<T> Drop for Immutree<T> {
    fn drop(&mut self) {
        for node in self.data.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<T> Immutree<T> {
    pub fn new() -> Immutree<T> {
        Immutree {
            data: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Immutree<T> {
        Immutree {
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn build_root_node<R>(
        &mut self,
        val: T,
        child_builder_cb: impl FnOnce(&mut ImmutreeNodeBuilder<T>) -> R,
    ) -> R {
        let mut node_builder = ImmutreeNodeBuilder {
            tree: self,
            last_added_child_index: None
        };
        node_builder.build_child(val, child_builder_cb)
    }

    pub fn iter(&self) -> ImmutreeNodeIter<T> {
        ImmutreeNodeIter {
            remaining_nodes: &self.data,
        }
    }

    pub fn iter_mut(&mut self) -> ImmutreeNodeIterMut<T> {
        ImmutreeNodeIterMut {
            remaining_nodes: &mut self.data[..]
        }
    }

    pub fn drain(mut self) -> ImmutreeDrain<T> {
        let data = std::mem::replace(&mut self.data, Vec::new());
        ImmutreeDrain { data, drop_from: 0 }
    }
}

struct NodeData<T> {
    val: T,
    next_sibling_offset: Option<NonZeroUsize> // Difference between the index of the next sibling and the index of the current node. None if there is no next sibling.
}

/// test
pub struct ImmutreeNodeBuilder<'a, T> {
    tree: &'a mut Immutree<T>,
    last_added_child_index: Option<usize>, // to update next_sibling_offset
}

impl<'a, T> ImmutreeNodeBuilder<'a, T> {
    pub fn add_leaf_child(&mut self, val: T) {
        self.build_child(val, |_| {});
    }

    pub fn build_child<R>(
        &mut self,
        val: T,
        child_builder_cb: impl FnOnce(&mut ImmutreeNodeBuilder<T>) -> R,
    ) -> R {
        let child_node_index = self.tree.data.len();
        self.tree.data.push(ManuallyDrop::new(NodeData {
            val,
            next_sibling_offset: None,
        }));

        // update next_sibling_offset of the last added node (if any)
        if let Some(last_added_child_index) = self.last_added_child_index {
            debug_assert!(last_added_child_index < child_node_index);
            let offset = child_node_index - last_added_child_index;
            debug_assert!(offset != 0);
            unsafe {
                self.tree.data.get_unchecked_mut(last_added_child_index).next_sibling_offset = Some(NonZeroUsize::new_unchecked(offset));
            }
        }
        self.last_added_child_index = Some(child_node_index);

        let mut child_node_builder = ImmutreeNodeBuilder {
            tree: self.tree,
            last_added_child_index: None,
        };
        child_builder_cb(&mut child_node_builder)
    }
}

/// test
pub struct ImmutreeNodeIter<'t, T> {
    remaining_nodes: &'t [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants
}

/// test
pub struct ImmutreeNodeRef<'t, T> {
    slice: &'t [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants
}

impl<'t, T> Iterator for ImmutreeNodeIter<'t, T> {
    type Item = ImmutreeNodeRef<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| {
                if let Some(next_sibling_offset) = cur_node.next_sibling_offset {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &[]);
                    let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at(next_sibling_offset.get());
                    self.remaining_nodes = next_nodes_slice;
                    ImmutreeNodeRef {
                        slice: cur_node_slice,
                    }
                } else {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &[]);
                    ImmutreeNodeRef {
                        slice: remaining_nodes,
                    }
                }
            })
    }
}

impl<'t, T> ImmutreeNodeRef<'t, T> {
    pub fn children(self) -> ImmutreeNodeIter<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first().unwrap();
        ImmutreeNodeIter { remaining_nodes }
    }

    pub fn val(&self) -> &T {
        debug_assert!(self.slice.len() > 0);
        unsafe { &self.slice.get_unchecked(0).val }
    }
}

pub struct ImmutreeNodeIterMut<'t, T> {
    remaining_nodes: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants
}

pub struct ImmutreeNodeRefMut<'t, T> {
    slice: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants
}

impl<'t, T> Iterator for ImmutreeNodeIterMut<'t, T> {
    type Item = ImmutreeNodeRefMut<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| cur_node.next_sibling_offset)
            .map(|maybe_next_sibling_offset| {
                if let Some(next_sibling_offset) = maybe_next_sibling_offset {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_sibling_offset.get());
                    self.remaining_nodes = next_nodes_slice;
                    ImmutreeNodeRefMut {
                        slice: cur_node_slice,
                    }
                } else {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    ImmutreeNodeRefMut {
                        slice: remaining_nodes,
                    }
                }
            })
    }
}

impl<'t, T> ImmutreeNodeRefMut<'t, T> {
    pub fn children(self) -> ImmutreeNodeIterMut<'t, T> {
        let (_, remaining_nodes) = self.slice.split_first_mut().unwrap();
        ImmutreeNodeIterMut { remaining_nodes }
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

pub struct ImmutreeDrain<T> {
    data: Vec<ManuallyDrop<NodeData<T>>>, // only data from `drop_from` to the end of the vec is dropped
    drop_from: usize,
}

pub struct ImmutreeNodeListDrain<'t, T> {
    remaining_nodes: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the nodes in the iterator and all their descendants. Drops them in drop().
}

pub struct ImmutreeSingleNodeDrain<'t, T> {
    slice: &'t mut [ManuallyDrop<NodeData<T>>], // contains (only) the current node and all its descendants. Drops them in drop().
}

impl<T> Drop for ImmutreeDrain<T> {
    fn drop(&mut self) {
        for node in self.data[self.drop_from..].iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<'t, T> Drop for ImmutreeNodeListDrain<'t, T> {
    fn drop(&mut self) {
        for node in self.remaining_nodes.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<'t, T> Drop for ImmutreeSingleNodeDrain<'t, T> {
    fn drop(&mut self) {
        for node in self.slice.iter_mut() {
            unsafe {
                ManuallyDrop::drop(node);
            }
        }
    }
}

impl<T> ImmutreeDrain<T> {
    pub fn drain_all(&mut self) -> ImmutreeNodeListDrain<T> {
        let drop_from = self.drop_from;
        self.drop_from = self.data.len();
        ImmutreeNodeListDrain {
            remaining_nodes: &mut self.data[drop_from..],
        }
    }
}

impl<'t, T> Iterator for ImmutreeNodeListDrain<'t, T> {
    type Item = ImmutreeSingleNodeDrain<'t, T>;
    fn next(&mut self) -> Option<Self::Item> {
        self.remaining_nodes
            .get(0)
            .map(|cur_node| cur_node.next_sibling_offset)
            .map(|maybe_next_sibling_offset| {
                if let Some(next_sibling_offset) = maybe_next_sibling_offset {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    let (cur_node_slice, next_nodes_slice) = remaining_nodes.split_at_mut(next_sibling_offset.get());
                    self.remaining_nodes = next_nodes_slice;
                    ImmutreeSingleNodeDrain {
                        slice: cur_node_slice,
                    }
                } else {
                    let remaining_nodes = std::mem::replace(&mut self.remaining_nodes, &mut []);
                    ImmutreeSingleNodeDrain {
                        slice: remaining_nodes,
                    }
                }
            })
    }
}

impl<'t, T> ImmutreeSingleNodeDrain<'t, T> {
    pub fn into_val_and_children(mut self) -> (T, ImmutreeNodeListDrain<'t, T>) {
        let slice = std::mem::replace(&mut self.slice, &mut []);
        let (node_data_ref, remaining_nodes) = slice.split_first_mut().unwrap();
        unsafe {
            let node_data: NodeData<T> = std::ptr::read(&**node_data_ref); // TODO: replace with ManuallyDrop::take() once that is stabilized
            (node_data.val, ImmutreeNodeListDrain { remaining_nodes })
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
