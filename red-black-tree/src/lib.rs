use bytemuck::{Pod, Zeroable};
use node_allocator::{NodeAllocator, ZeroCopy, SENTINEL};
use std::ops::{Index, IndexMut};

// Register aliases
pub const LEFT: u32 = 0;
pub const RIGHT: u32 = 1;
pub const PARENT: u32 = 2;
pub const COLOR: u32 = 3;
// Default nodes are conveniently colored black
pub const BLACK: u32 = 0;
pub const RED: u32 = 1;

/// Exploits the fact that LEFT and RIGHT are set to 0 and 1 respectively
#[inline(always)]
fn opposite(dir: u32) -> u32 {
    1 - dir
}

#[repr(C)]
#[derive(Default, Copy, Clone)]
pub struct RBNode<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
> {
    pub key: K,
    pub value: V,
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Zeroable for RBNode<K, V>
{
}
unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > Pod for RBNode<K, V>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > RBNode<K, V>
{
    pub fn new(key: K, value: V) -> Self {
        Self { key, value }
    }
}

#[derive(Copy, Clone)]
pub struct RedBlackTree<
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub root: u32,
    allocator: NodeAllocator<RBNode<K, V>, MAX_SIZE, 4>,
}

unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Zeroable for RedBlackTree<K, V, MAX_SIZE>
{
}
unsafe impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Pod for RedBlackTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > ZeroCopy for RedBlackTree<K, V, MAX_SIZE>
{
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Default for RedBlackTree<K, V, MAX_SIZE>
{
    fn default() -> Self {
        RedBlackTree {
            root: SENTINEL,
            allocator: NodeAllocator::<RBNode<K, V>, MAX_SIZE, 4>::default(),
        }
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > RedBlackTree<K, V, MAX_SIZE>
{
    pub fn size(&self) -> usize {
        self.allocator.size as usize
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_slice(slice: &mut [u8]) -> &mut Self {
        let tree = Self::load_mut_bytes(slice).unwrap();
        tree.allocator.init_default();
        tree
    }

    pub fn get_node(&self, node: u32) -> &RBNode<K, V> {
        self.allocator.get(node).get_value()
    }

    fn get_node_mut(&mut self, node: u32) -> &mut RBNode<K, V> {
        self.allocator.get_mut(node).get_value_mut()
    }

    #[inline(always)]
    fn color_red(&mut self, node: u32) {
        if node != SENTINEL {
            self.allocator.set_register(node, RED, COLOR);
        }
    }

    #[inline(always)]
    fn color_black(&mut self, node: u32) {
        self.allocator.set_register(node, BLACK, COLOR);
    }

    #[inline(always)]
    fn is_red(&self, node: u32) -> bool {
        self.allocator.get_register(node, COLOR) == RED
    }

    #[inline(always)]
    fn is_black(&self, node: u32) -> bool {
        self.allocator.get_register(node, COLOR) == BLACK
    }

    #[inline(always)]
    fn get_child(&self, node: u32, dir: u32) -> u32 {
        self.allocator.get_register(node, dir)
    }

    #[inline(always)]
    pub fn is_leaf(&self, node: u32) -> bool {
        self.get_left(node) == SENTINEL && self.get_right(node) == SENTINEL
    }

    #[inline(always)]
    pub fn get_left(&self, node: u32) -> u32 {
        self.allocator.get_register(node, LEFT)
    }

    #[inline(always)]
    pub fn get_right(&self, node: u32) -> u32 {
        self.allocator.get_register(node, RIGHT)
    }

    #[inline(always)]
    pub fn get_color(&self, node: u32) -> u32 {
        self.allocator.get_register(node, COLOR)
    }

    #[inline(always)]
    pub fn get_parent(&self, node: u32) -> u32 {
        self.allocator.get_register(node, PARENT)
    }

    #[inline(always)]
    pub fn connect(&mut self, parent: u32, child: u32, dir: u32) {
        self.allocator.connect(parent, child, dir, PARENT);
    }

    #[inline(always)]
    fn child_dir(&self, parent: u32, child: u32) -> u32 {
        let left = self.get_left(parent);
        let right = self.get_right(parent);
        if child == left {
            assert!(self.get_parent(child) == parent);
            LEFT
        } else if child == right {
            assert!(self.get_parent(child) == parent);
            RIGHT
        } else {
            panic!("Nodes are not connected");
        }
    }

    fn rotate_dir(&mut self, parent_index: u32, dir: u32) -> Option<u32> {
        let grandparent_index = self.get_parent(parent_index);
        match dir {
            LEFT | RIGHT => {}
            _ => return None,
        }
        let sibling_index = self.get_child(parent_index, opposite(dir));
        if sibling_index == SENTINEL {
            return None;
        }
        let child_index = self.get_child(sibling_index, dir);
        self.connect(sibling_index, parent_index, dir);
        self.connect(parent_index, child_index, opposite(dir));
        if grandparent_index != SENTINEL {
            if self.get_left(grandparent_index) == parent_index {
                self.connect(grandparent_index, sibling_index, LEFT);
            } else if self.get_right(grandparent_index) == parent_index {
                self.connect(grandparent_index, sibling_index, RIGHT);
            } else {
                return None;
            }
        } else {
            self.allocator.clear_register(sibling_index, PARENT);
            self.root = sibling_index;
        }
        Some(sibling_index)
    }

    fn fix_insert(&mut self, mut node: u32) -> Option<()> {
        while self.is_red(self.get_parent(node)) {
            let mut parent = self.get_parent(node);
            let mut grandparent = self.get_parent(parent);
            if grandparent == SENTINEL {
                assert!(parent == self.root);
                break;
            }
            let dir = self.child_dir(grandparent, parent);
            let uncle = self.get_child(grandparent, opposite(dir));
            if self.is_red(uncle) {
                self.color_black(uncle);
                self.color_black(parent);
                self.color_red(grandparent);
                node = grandparent;
            } else {
                if self.child_dir(parent, node) == opposite(dir) {
                    self.rotate_dir(node, dir);
                    node = parent;
                }
                parent = self.get_parent(node);
                grandparent = self.get_parent(parent);
                self.color_black(parent);
                self.color_red(grandparent);
                self.rotate_dir(grandparent, opposite(dir));
            }
        }
        self.color_black(self.root);
        Some(())
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<u32> {
        let mut reference_node = self.root;
        let new_node = RBNode::<K, V>::new(key, value);
        if reference_node == SENTINEL {
            let node_index = self.allocator.add_node(new_node);
            self.root = node_index;
            return Some(node_index);
        }
        loop {
            let ref_value = self.get_node(reference_node).key;
            let (target, dir) = if key < ref_value {
                (self.get_left(reference_node), LEFT)
            } else if key > ref_value {
                (self.get_right(reference_node), RIGHT)
            } else {
                self.get_node_mut(reference_node).value = value;
                return Some(reference_node);
            };
            if target == SENTINEL {
                if self.size() >= MAX_SIZE - 1 {
                    return None;
                }
                let node_index = self.allocator.add_node(new_node);
                self.color_red(node_index);
                self.connect(reference_node, node_index, dir);
                let grandparent = self.get_parent(reference_node);
                if grandparent != SENTINEL {
                    self.fix_insert(node_index);
                }
                return Some(node_index);
            }
            reference_node = target
        }
    }

    fn fix_remove(&mut self, mut node_index: u32) -> Option<()> {
        if node_index == SENTINEL {
            return Some(());
        }
        while node_index != self.root && self.is_black(node_index) {
            let parent = self.get_parent(node_index);
            let dir = self.child_dir(parent, node_index);
            let mut sibling = self.get_child(parent, opposite(dir));
            if self.is_red(sibling) {
                self.color_black(sibling);
                self.color_red(parent);
                self.rotate_dir(parent, dir);
                sibling = self.get_right(self.get_parent(node_index));
            }
            if self.is_black(self.get_left(sibling)) && self.is_black(self.get_right(sibling)) {
                self.color_red(sibling);
                node_index = self.get_parent(node_index);
            } else {
                if self.is_black(self.get_right(sibling)) {
                    self.color_black(self.get_left(sibling));
                    self.color_red(sibling);
                    self.rotate_dir(sibling, opposite(dir));
                    sibling = self.get_right(self.get_parent(node_index));
                }

                let parent = self.get_parent(node_index);
                if self.is_red(parent) {
                    self.color_red(sibling);
                } else {
                    self.color_black(sibling);
                }
                self.color_black(parent);
                self.color_black(self.get_right(sibling));
                self.rotate_dir(parent, dir);
                node_index = self.root;
            }
        }
        Some(())
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut ref_node_index = self.root;
        if ref_node_index == SENTINEL {
            return None;
        }
        loop {
            let ref_key = self.allocator.get(ref_node_index).get_value().key;
            let ref_value = self.allocator.get(ref_node_index).get_value().value;
            let left = self.get_left(ref_node_index);
            let right = self.get_right(ref_node_index);
            let target = if *key < ref_key {
                left
            } else if *key > ref_key {
                right
            } else {
                let mut is_black = self.is_black(ref_node_index);
                let (pivot_node_index, delete_node_index) = if left == SENTINEL {
                    self.transplant(ref_node_index, right);
                    self.allocator.clear_register(ref_node_index, RIGHT);
                    (right, ref_node_index)
                } else if right == SENTINEL {
                    self.transplant(ref_node_index, left);
                    self.allocator.clear_register(ref_node_index, LEFT);
                    (left, ref_node_index)
                } else {
                    assert!(self.get_parent(self.get_left(ref_node_index)) == ref_node_index);
                    assert!(self.get_parent(self.get_right(ref_node_index)) == ref_node_index);
                    let min_right = self.find_min(right);
                    let min_right_child = self.get_right(min_right);
                    is_black = self.is_black(min_right);
                    if min_right == right {
                        assert!(
                            min_right_child == SENTINEL
                                || self.get_parent(min_right_child) == right
                        );
                    } else {
                        self.transplant(min_right, min_right_child);
                        self.connect(min_right, right, RIGHT);
                    }
                    self.transplant(ref_node_index, min_right);
                    self.connect(min_right, left, LEFT);
                    if self.is_red(ref_node_index) {
                        self.color_red(min_right)
                    } else {
                        self.color_black(min_right)
                    }
                    self.allocator.clear_register(ref_node_index, LEFT);
                    self.allocator.clear_register(ref_node_index, RIGHT);
                    (min_right_child, ref_node_index)
                };
                self.allocator.clear_register(ref_node_index, PARENT);
                assert!(self.is_leaf(delete_node_index));
                self.allocator.clear_register(delete_node_index, COLOR);
                self.allocator.remove_node(delete_node_index);
                if is_black {
                    if self.fix_remove(pivot_node_index) == None {
                        return None;
                    }
                }
                return Some(ref_value);
            };
            if target == SENTINEL {
                println!("Key not found in tree");
                return None;
            }
            ref_node_index = target
        }
    }

    fn transplant(&mut self, target: u32, source: u32) {
        let parent = self.get_parent(target);
        if parent == SENTINEL {
            self.root = source;
            self.allocator.set_register(source, SENTINEL, PARENT);
            return;
        }
        let dir = self.child_dir(parent, target);
        self.connect(parent, source, dir);
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        let mut reference_node = self.root;
        if reference_node == SENTINEL {
            return None;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_left(reference_node)
            } else if *key > ref_value {
                self.get_right(reference_node)
            } else {
                return Some(&self.get_node(reference_node).value);
            };
            if target == SENTINEL {
                return None;
            }
            reference_node = target
        }
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let mut reference_node = self.root;
        if reference_node == SENTINEL {
            return None;
        }
        loop {
            let ref_value = self.allocator.get(reference_node).get_value().key;
            let target = if *key < ref_value {
                self.get_left(reference_node)
            } else if *key > ref_value {
                self.get_right(reference_node)
            } else {
                return Some(&mut self.get_node_mut(reference_node).value);
            };
            if target == SENTINEL {
                return None;
            }
            reference_node = target
        }
    }

    pub fn find_min(&self, index: u32) -> u32 {
        let mut node = index;
        while self.get_left(node) != SENTINEL {
            node = self.get_left(node);
        }
        node
    }

    pub fn find_max(&self, index: u32) -> u32 {
        let mut node = index;
        while self.get_right(node) != SENTINEL {
            node = self.get_right(node);
        }
        node
    }

    pub fn iter(&self) -> RedBlackTreeIterator<'_, K, V, MAX_SIZE> {
        RedBlackTreeIterator::<K, V, MAX_SIZE> {
            tree: self,
            stack: vec![],
            node: self.root,
        }
    }

    pub fn iter_mut(&mut self) -> RedBlackTreeIteratorMut<'_, K, V, MAX_SIZE> {
        let node = self.root;
        RedBlackTreeIteratorMut::<K, V, MAX_SIZE> {
            tree: self,
            stack: vec![],
            node,
        }
    }
}

pub struct RedBlackTreeIterator<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub tree: &'a RedBlackTree<K, V, MAX_SIZE>,
    pub stack: Vec<u32>,
    pub node: u32,
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for RedBlackTreeIterator<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.stack.push(self.node);
                self.node = self.tree.get_left(self.node);
            } else {
                self.node = self.stack.pop().unwrap();
                let node = self.tree.get_node(self.node);
                self.node = self.tree.get_right(self.node);
                return Some((&node.key, &node.value));
            }
        }
        return None;
    }
}

pub struct RedBlackTreeIteratorMut<
    'a,
    K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
    V: Default + Copy + Clone + Pod + Zeroable,
    const MAX_SIZE: usize,
> {
    pub tree: &'a mut RedBlackTree<K, V, MAX_SIZE>,
    pub stack: Vec<u32>,
    pub node: u32,
}

impl<
        'a,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Iterator for RedBlackTreeIteratorMut<'a, K, V, MAX_SIZE>
{
    type Item = (&'a K, &'a mut V);

    fn next(&mut self) -> Option<Self::Item> {
        while !self.stack.is_empty() || self.node != SENTINEL {
            if self.node != SENTINEL {
                self.stack.push(self.node);
                self.node = self.tree.get_left(self.node);
            } else {
                self.node = self.stack.pop().unwrap();
                let ptr = self.node;
                self.node = self.tree.get_right(ptr);
                // TODO: How does one remove this unsafe?
                unsafe {
                    let node =
                        (*self.tree.allocator.nodes.as_mut_ptr().add(ptr as usize)).get_value_mut();
                    return Some((&node.key, &mut node.value));
                }
            }
        }
        return None;
    }
}

impl<
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
        const MAX_SIZE: usize,
    > Index<&K> for RedBlackTree<K, V, MAX_SIZE>
{
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        &self.get(index).unwrap()
    }
}

impl<
        const MAX_SIZE: usize,
        K: PartialOrd + Copy + Clone + Default + Pod + Zeroable,
        V: Default + Copy + Clone + Pod + Zeroable,
    > IndexMut<&K> for RedBlackTree<K, V, MAX_SIZE>
{
    fn index_mut(&mut self, index: &K) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}
