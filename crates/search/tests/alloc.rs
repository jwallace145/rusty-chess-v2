//! The post-mortem's guard against allocation in the search path: a counting
//! allocator wraps the system allocator, and a search may allocate only a
//! small fixed amount per iteration (the iteration record and its strings),
//! never per node. Lesson 7 of docs/lessons-from-rusty-chess-v1.md.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rusty_chess_core::position::Position;
use rusty_chess_search::board::Board;
use rusty_chess_search::search::{Limits, Searcher};

struct Counting;

static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static GLOBAL: Counting = Counting;

#[test]
fn the_search_path_does_not_allocate_per_node() {
    rusty_chess_core::attacks::init();
    let fen = "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1";
    let mut board = Board::from_position(Position::from_fen(fen).unwrap());
    let mut searcher = Searcher::new(Arc::new(AtomicBool::new(false)));
    let depth = 6;
    let limits = Limits {
        depth: Some(depth),
        ..Limits::default()
    };

    let before = ALLOCATIONS.load(Ordering::Relaxed);
    let result = searcher.search(&mut board, &limits, &mut |_| {});
    let allocations = ALLOCATIONS.load(Ordering::Relaxed) - before;

    let nodes = result.nodes + result.qnodes;
    assert!(
        nodes > 100_000,
        "the search should be large enough to expose per-node allocation: {nodes} nodes"
    );
    // Per iteration: the root FEN string, the PV string and its pieces, the
    // record pushed into the iterations vector. Generous, and still four
    // orders of magnitude below one allocation per node.
    let budget = 32 * depth as usize + 32;
    assert!(
        allocations <= budget,
        "{allocations} allocations during a {nodes}-node search (budget {budget}): something allocates per node"
    );
}
