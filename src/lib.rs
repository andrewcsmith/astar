#![feature(rustc_private, collections)]

extern crate num;
extern crate arena;

use arena::TypedArena;
use num::Zero;
use std::hash::Hash;
use std::collections::BinaryHeap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::collections::HashMap;
use std::cmp::Ordering;
use std::cell::{Cell, RefCell};
use std::mem;

pub trait SearchProblem {
    type Node: Hash + PartialEq + Eq;
    type Cost: PartialOrd + Zero + Clone;
    type Iter: Iterator<Item = (Self::Node, Self::Cost)>;

    fn start(&self) -> Self::Node;
    fn is_end(&self, &Self::Node) -> bool;
    fn heuristic(&self, &Self::Node) -> Self::Cost;
    fn neighbors(&mut self, &Self::Node) -> Self::Iter;

    fn estimate_length(&self) -> Option<u64> { None }
}

struct SearchNode<'a: 'b, 'b, S: 'a , C: Clone + 'a> {
    pub state: &'a S,
    pub parent: RefCell<Option<&'b SearchNode<'a, 'b, S, C>>>,

    pub g: RefCell<C>,
    pub f: RefCell<C>,
    pub h: RefCell<C>,

    pub opened: Cell<bool>,
    pub closed: Cell<bool>,
}

impl <'a, 'b, S, C: Zero + Clone> SearchNode<'a, 'b, S, C> {
    fn new_initial(state: &'a S) -> SearchNode<S, C> {
        SearchNode {
            state: state,
            parent: RefCell::new(None),
            g: RefCell::new(Zero::zero()),
            f: RefCell::new(Zero::zero()),
            h: RefCell::new(Zero::zero()),
            opened: Cell::new(true),
            closed: Cell::new(false)
        }
    }

    fn new(state: &'a S) -> SearchNode<S, C> {
        SearchNode {
            state: state,
            parent: RefCell::new(None),
            g: RefCell::new(Zero::zero()),
            f: RefCell::new(Zero::zero()),
            h: RefCell::new(Zero::zero()),
            opened: Cell::new(false),
            closed: Cell::new(false)
        }
    }

    fn g(&self) -> C {
        self.g.borrow().clone()
    }

    fn h(&self) -> C {
        self.h.borrow().clone()
    }

    fn set_g(&self, g: C) {
        *self.g.borrow_mut() = g;
    }

    fn set_f(&self, f: C) {
        *self.f.borrow_mut() = f;
    }

    fn set_h(&self, h: C) {
        *self.h.borrow_mut() = h;
    }

    fn set_parent(&self, p: &'b SearchNode<'a, 'b, S, C>) {
        *self.parent.borrow_mut() = Some(p);
    }
}

impl <'a, 'b, S: PartialEq, C: Clone> PartialEq for SearchNode<'a, 'b, S, C> {
    fn eq(&self, other: &SearchNode<S, C>) -> bool {
        self.state.eq(&other.state)
    }
}

impl <'a, 'b, S: PartialEq, C: Clone> Eq for SearchNode<'a, 'b, S, C> {}

impl<'a, 'b, S: PartialEq, C: PartialOrd + Clone> PartialOrd for SearchNode<'a, 'b, S, C> {
    fn partial_cmp(&self, other: &SearchNode<S, C>) -> Option<Ordering> {
        other.f.borrow().partial_cmp(&self.f.borrow())
    }
}

impl<'a, 'b, S: PartialEq, C: PartialOrd + Clone> Ord for SearchNode<'a, 'b, S, C> {
    fn cmp(&self, other: &SearchNode<'a, 'b, S, C>) -> Ordering {
        match self.partial_cmp(other) {
            Some(x) => x,
            None => Ordering::Equal
        }
    }
}

pub fn astar<S: SearchProblem>(s: &mut S) -> Option<VecDeque<S::Node>> {
    let state_arena = TypedArena::new();
    let node_arena = TypedArena::new();

    let mut state_to_node = HashMap::new();
    let mut closed = HashSet::new();

    let mut heap = BinaryHeap::new();

    let start_state:&_ = state_arena.alloc(s.start());

    let start_node: SearchNode<S::Node, S::Cost> = SearchNode::new_initial(start_state);
    let start_node:&_ = node_arena.alloc(start_node);
    state_to_node.insert(start_state, start_node);

    heap.push(start_node);

    let mut found = None;

    while let Some(node) = heap.pop() {
        let node_state = node.state;
        closed.insert(node_state);

        node.closed.set(true);

        if s.is_end(node_state) {
            found = Some(node)
        }

        for (neighbor, cost) in s.neighbors(node_state) {
            let neighbor_state:&_ = state_arena.alloc(neighbor);
            let neighbor_node = state_to_node.get(neighbor_state).cloned()
                                             .unwrap_or_else(|| {
                node_arena.alloc(SearchNode::new(neighbor_state))
            });

            let ng = node.g() + cost;
            if !neighbor_node.opened.get() || ng < neighbor_node.g() {

                let h = if neighbor_node.h() == Zero::zero() {
                    s.heuristic(neighbor_state)
                } else { neighbor_node.h() };

                neighbor_node.set_g(ng.clone());
                neighbor_node.set_h(h.clone());
                neighbor_node.set_f(ng + h);
                // TODO: set parent
                neighbor_node.set_parent(node);

                if !neighbor_node.opened.get() {
                    neighbor_node.opened.set(true);
                    heap.push(neighbor_node);
                } else {
                    // We reset the value that did sorting.  This forces a
                    // recalculation.
                    heap = BinaryHeap::from_vec(heap.into_vec());
                }
            }
        }
    }

    if found.is_some() {
        let mut prev = found;
        let mut deque = VecDeque::new();

        while let Some(node) = prev {
            unsafe {
                deque.push_front(mem::replace(mem::transmute(node.state),
                                              mem::uninitialized()));
            }
            prev = node.parent.borrow_mut().take();
        }
        Some(deque)

    } else {
        None
    }
}
