//! Quick sort using the Lomuto partition scheme (last element as pivot).
//!
//! Event story:
//! * Each partition pins the pivot with
//!   [`SetPivot`](crate::simulation::SortEvent::SetPivot) (magenta).
//! * Every element is compared against the pivot
//!   ([`Compare`](crate::simulation::SortEvent::Compare)) and smaller elements
//!   are swapped toward the front ([`Swap`](crate::simulation::SortEvent::Swap)).
//! * After the pivot is moved to its boundary it is in its *final* position, so
//!   it is marked sorted. Singleton sub-ranges are marked sorted directly.
//! * [`ClearHighlights`](crate::simulation::SortEvent::ClearHighlights) is
//!   emitted at the end of each partition to wipe the pivot color before the
//!   next one.
//!
//! Indices are tracked as `isize` so an empty right/left sub-range (`p - 1`
//! when `p == 0`) is expressible without underflow.

use super::{Recorder, SortingAlgorithm};
use crate::simulation::SortEvent;

pub struct QuickSort;

impl SortingAlgorithm for QuickSort {
    fn name(&self) -> &'static str {
        "Quick Sort"
    }

    fn generate_events(&self, values: &[u32]) -> Vec<SortEvent> {
        let mut a = values.to_vec();
        let n = a.len();
        let mut rec = Recorder::with_capacity_for(n);

        if n <= 1 {
            if n == 1 {
                rec.mark_sorted(0);
            }
            return rec.finish();
        }

        quicksort(&mut a, 0, n as isize - 1, &mut rec);
        rec.finish()
    }
}

/// Recursively sort the inclusive range `[lo, hi]`, recording events.
fn quicksort(a: &mut [u32], lo: isize, hi: isize, rec: &mut Recorder) {
    if lo > hi {
        return; // empty range
    }
    if lo == hi {
        rec.mark_sorted(lo as usize); // singleton is trivially in place
        return;
    }

    let pivot_pos = partition(a, lo, hi, rec);
    rec.mark_sorted(pivot_pos as usize);

    quicksort(a, lo, pivot_pos - 1, rec);
    quicksort(a, pivot_pos + 1, hi, rec);
}

/// Lomuto partition over `[lo, hi]`, returning the pivot's final index.
fn partition(a: &mut [u32], lo: isize, hi: isize, rec: &mut Recorder) -> isize {
    let pivot_index = hi as usize;
    rec.set_pivot(pivot_index);
    let pivot = a[pivot_index];

    // `i` marks the boundary: everything in [lo, i) is known to be < pivot.
    let mut i = lo;
    for j in lo..hi {
        rec.compare(j as usize, pivot_index);
        if a[j as usize] < pivot {
            if i != j {
                rec.swap(a, i as usize, j as usize);
            }
            i += 1;
        }
    }

    // Move the pivot into its sorted slot at `i`.
    if i as usize != pivot_index {
        rec.swap(a, i as usize, pivot_index);
    }
    rec.clear_highlights();
    i
}
