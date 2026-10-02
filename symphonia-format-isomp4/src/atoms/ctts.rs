// Symphonia
// Copyright (c) 2019-2026 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::atoms::limits::*;
use crate::atoms::{Atom, AtomHeader, AtomIterator, ReadAtom, Result, decode_error};

/// Composition time-to-sample entry.
#[derive(Debug)]
pub struct CttsEntry {
    /// Absolute sample number of the first sample described by this entry.
    pub first_sample: u32,
    /// Number of samples described by this entry.
    pub sample_count: u32,
    /// Composition offset of the described samples relative to their decode timestamp.
    pub sample_offset: i64,
}

/// Composition time-to-sample atom.
#[derive(Debug)]
pub struct CttsAtom {
    pub entries: Vec<CttsEntry>,
}

impl CttsAtom {
    /// Get the composition offset of the sample indicated by `sample_num`. Note, `sample_num` is
    /// an absolute sample number.
    pub fn offset_for_sample(&self, sample_num: u32) -> Option<i64> {
        let entry_idx = self.entries.partition_point(|e| e.first_sample <= sample_num);
        let entry = self.entries.get(entry_idx.checked_sub(1)?)?;

        if sample_num < entry.first_sample.saturating_add(entry.sample_count) {
            Some(entry.sample_offset)
        }
        else {
            None
        }
    }
}

impl Atom for CttsAtom {
    fn read<R: ReadAtom>(it: &mut AtomIterator<R>, _header: &AtomHeader) -> Result<Self> {
        let (version, _) = it.read_extended_header()?;

        if version > 1 {
            return decode_error("isomp4 (ctts): invalid ctts version");
        }

        let entry_count = it.read_u32()?;

        // Limit the maximum initial capacity to prevent malicious files from using all the
        // available memory.
        let mut entries = Vec::with_capacity(MAX_TABLE_INITIAL_CAPACITY.min(entry_count as usize));

        let mut first_sample = 0;

        for _ in 0..entry_count {
            let sample_count = it.read_u32()?;

            let sample_offset = match version {
                1 => i64::from(it.read_i32()?),
                _ => i64::from(it.read_u32()?),
            };

            entries.push(CttsEntry {
                first_sample,
                sample_count,
                sample_offset,
            });

            first_sample = first_sample.saturating_add(sample_count);
        }

        Ok(CttsAtom { entries })
    }
}
