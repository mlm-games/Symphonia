// Symphonia
// Copyright (c) 2019-2026 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::atoms::{
    Atom, AtomHeader, AtomIterator, AtomType, EdtsAtom, MdiaAtom, ReadAtom, Result, TkhdAtom,
    decode_error,
};

/// Track atom.
#[allow(dead_code)]
#[derive(Debug)]
pub struct TrakAtom {
    /// Track header atom.
    pub tkhd: TkhdAtom,
    /// Optional, edit list atom.
    pub edts: Option<EdtsAtom>,
    /// Media atom.
    pub mdia: MdiaAtom,
}

impl TrakAtom {
    /// Get the number of timescale units skipped from the start of the media by the first edit of
    /// this track.
    pub fn media_start_time(&self) -> i64 {
        self.edts
            .as_ref()
            .and_then(|edts| edts.elst.as_ref())
            .map_or(0, |elst| elst.first_media_time())
    }

    /// Get the presentation timestamp of the sample with decode timestamp `dts`.
    pub fn presentation_timestamp(&self, dts: u64, sample_num: u32) -> i64 {
        i64::try_from(dts)
            .unwrap_or(i64::MAX)
            .saturating_add(self.mdia.minf.stbl.composition_offset(sample_num))
            .saturating_sub(self.media_start_time())
    }

    /// Get the sample number of the sample whose presentation timestamp is contained by `pts`.
    /// Note, the returned sample number is indexed relative to the track. Complexity of this
    /// function in O(N).
    pub fn find_sample_for_pts(&self, pts: u64) -> Option<u32> {
        let stbl = &self.mdia.minf.stbl;
        let target = i64::try_from(pts).unwrap_or(i64::MAX).saturating_add(self.media_start_time());

        let mut sample_num = 0;
        let mut dts: u64 = 0;

        for entry in &stbl.stts.entries {
            let delta = u64::from(entry.sample_delta);

            for _ in 0..entry.sample_count {
                let sample_pts = i64::try_from(dts)
                    .unwrap_or(i64::MAX)
                    .saturating_add(stbl.composition_offset(sample_num));

                if sample_pts.saturating_add(i64::from(entry.sample_delta)) > target {
                    return Some(sample_num);
                }

                dts = dts.saturating_add(delta);
                sample_num += 1;
            }
        }

        None
    }
}

impl Atom for TrakAtom {
    fn read<R: ReadAtom>(it: &mut AtomIterator<R>, _header: &AtomHeader) -> Result<Self> {
        let mut tkhd = None;
        let mut edts = None;
        let mut mdia = None;

        while let Some(header) = it.next_header()? {
            match header.atom_type {
                AtomType::TrackHeader => {
                    tkhd = Some(it.read_atom::<TkhdAtom>()?);
                }
                AtomType::Edit => {
                    edts = Some(it.read_atom::<EdtsAtom>()?);
                }
                AtomType::Media => {
                    mdia = Some(it.read_atom::<MdiaAtom>()?);
                }
                _ => (),
            }
        }

        let Some(tkhd) = tkhd
        else {
            return decode_error("isomp4 (trak): missing tkhd atom");
        };

        let Some(mdia) = mdia
        else {
            return decode_error("isomp4 (trak): missing mdia atom");
        };

        Ok(TrakAtom { tkhd, edts, mdia })
    }
}
