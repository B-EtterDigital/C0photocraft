//! What the C0VIBE host needs of the session to show "sessions": named groups of documents shown as
//! tabs. This file is not upstream PhotoCraft's (see `c0/photocraft-host/README.md`).
//!
//! * A stable id per open document ([`DocState::uid`]): a process-wide counter, so an id is never
//!   reused in a run, whatever happens to tab order, to other documents, or to a document that is
//!   closed and opened again.
//! * A tab filter ([`Session::set_visible`]): the tab strip draws only the listed documents, Next and
//!   Previous Document and Window › Arrange go through them only, and a document opened while the
//!   filter is set joins it. Hidden documents stay open and untouched.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::{DocState, Session};

static NEXT_UID: AtomicU64 = AtomicU64::new(1);

/// A fresh document id.
pub(crate) fn next_uid() -> u64 {
    NEXT_UID.fetch_add(1, Ordering::Relaxed)
}

impl Session {
    /// The tab filter: the ids whose tabs show, `None` when every tab does.
    pub fn visible(&self) -> Option<&[u64]> {
        self.visible.as_deref()
    }

    /// Where the document with id `uid` is in tab order.
    pub fn index_of_uid(&self, uid: u64) -> Option<usize> {
        self.docs.iter().position(|d| d.uid == uid)
    }

    /// The document with id `uid`.
    pub fn by_uid(&self, uid: u64) -> Option<&DocState> {
        self.docs.iter().find(|d| d.uid == uid)
    }

    /// Whether the tab at `index` shows under the filter.
    pub fn tab_visible(&self, index: usize) -> bool {
        match (&self.visible, self.docs.get(index)) {
            (Some(list), Some(d)) => list.contains(&d.uid),
            (None, Some(_)) => true,
            _ => false,
        }
    }

    /// The tabs that show, in tab order.
    pub fn visible_indices(&self) -> Vec<usize> {
        (0..self.docs.len()).filter(|&i| self.tab_visible(i)).collect()
    }

    /// Sets the tab filter (`None`: show all). Every id must be an open document; a repeated id counts
    /// once. When the active document is not shown, the first listed one becomes active.
    pub fn set_visible(&mut self, list: Option<Vec<u64>>) -> Result<(), String> {
        let list = match list {
            None => None,
            Some(ids) => {
                let mut kept: Vec<u64> = Vec::with_capacity(ids.len());
                for id in ids {
                    if self.index_of_uid(id).is_none() {
                        return Err(format!("no document with id {id}"));
                    }
                    if !kept.contains(&id) {
                        kept.push(id);
                    }
                }
                Some(kept)
            }
        };
        self.visible = list;
        self.settle_visible();
        Ok(())
    }

    /// A document was added at `index`: it joins the filter.
    pub(crate) fn join_visible(&mut self, index: usize) {
        if let (Some(list), Some(d)) = (self.visible.as_mut(), self.docs.get(index)) {
            list.push(d.uid);
        }
    }

    /// Drops closed documents from the filter and, when the active document is not shown, activates
    /// the first listed one that is open.
    pub(crate) fn settle_visible(&mut self) {
        let Some(list) = self.visible.as_mut() else { return };
        let open: Vec<u64> = self.docs.iter().map(|d| d.uid).collect();
        list.retain(|id| open.contains(id));
        let active_shown = self.active.is_some_and(|i| self.docs.get(i).is_some_and(|d| list.contains(&d.uid)));
        if !active_shown && let Some(first) = list.first().and_then(|id| self.docs.iter().position(|d| d.uid == *id)) {
            self.active = Some(first);
        }
    }

    /// The document Next (`forward`) or Previous Document goes to from `at`, wrapping, among the shown
    /// tabs only. From a tab that is not shown: the first shown tab after (before) it.
    pub fn cycle_visible(&self, at: usize, forward: bool) -> usize {
        let n = self.docs.len();
        for step in 1..=n {
            let to = if forward { (at + step) % n } else { (at + n - step % n) % n };
            if self.tab_visible(to) {
                return to;
            }
        }
        at
    }

    /// The documents Window › Arrange tiles: the shown ones, in the order they are laid out, starting
    /// with the active document `active`.
    pub fn arrange_order(&self, active: usize) -> Vec<usize> {
        let n = self.docs.len();
        (0..n).map(|k| (active + k) % n).filter(|&i| self.tab_visible(i)).collect()
    }
}
