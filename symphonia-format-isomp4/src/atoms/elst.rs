// Symphonia
// Copyright (c) 2019-2026 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::util::bits;

use crate::atoms::{Atom, AtomHeader, AtomIterator, ReadAtom, Result};
use crate::atoms::{decode_error, limits::*};

/// Edit list entry.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ElstEntry {
    segment_duration: u64,
    media_time: i64,
    media_rate_int: i16,
    media_rate_frac: i16,
}

/// Edit list atom.
#[allow(dead_code)]
#[derive(Debug)]
pub struct ElstAtom {
    entries: Vec<ElstEntry>,
}

impl ElstAtom {
    /// Get the media time, in media timescale units, at which the presentation of this track
    /// starts. Leading empty edits delay the presentation; their segment duration is expressed
    /// in movie timescale units and is converted to media timescale units here.
    pub fn first_media_time(&self, movie_timescale: u32, media_timescale: u32) -> i64 {
        let media_timescale = i128::from(media_timescale);
        let movie_timescale = i128::from(movie_timescale.max(1));

        let mut empty_duration = 0i128;

        for entry in &self.entries {
            if entry.media_time >= 0 {
                let start = i128::from(entry.media_time) - empty_duration;
                return start.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
            }

            empty_duration +=
                i128::from(entry.segment_duration) * media_timescale / movie_timescale;
        }

        0
    }
}

impl Atom for ElstAtom {
    fn read<R: ReadAtom>(it: &mut AtomIterator<R>, _header: &AtomHeader) -> Result<Self> {
        let (version, _) = it.read_extended_header()?;

        if version > 1 {
            return decode_error("isomp4 (elst): invalid tkhd version");
        }

        let entry_count = it.read_u32()?;

        // Limit the maximum initial capacity to prevent malicious files from using all the
        // available memory.
        let mut entries = Vec::with_capacity(MAX_TABLE_INITIAL_CAPACITY.min(entry_count as usize));

        for _ in 0..entry_count {
            let (segment_duration, media_time) = match version {
                0 => (
                    u64::from(it.read_u32()?),
                    i64::from(bits::sign_extend_leq32_to_i32(it.read_u32()?, 32)),
                ),
                1 => (it.read_u64()?, bits::sign_extend_leq64_to_i64(it.read_u64()?, 64)),
                _ => unreachable!(),
            };

            let media_rate_int = bits::sign_extend_leq16_to_i16(it.read_u16()?, 16);
            let media_rate_frac = bits::sign_extend_leq16_to_i16(it.read_u16()?, 16);

            entries.push(ElstEntry {
                segment_duration,
                media_time,
                media_rate_int,
                media_rate_frac,
            });
        }

        Ok(ElstAtom { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::{ElstAtom, ElstEntry};

    fn edit(segment_duration: u64, media_time: i64) -> ElstEntry {
        ElstEntry { segment_duration, media_time, media_rate_int: 1, media_rate_frac: 0 }
    }

    #[test]
    fn first_media_time_accounts_for_leading_empty_edits() {
        // Empty edit spans 45000 movie ticks (== media ticks), then the media edit starts at
        // media_time 3000. Presentation must begin 45000-3000 ticks after the first sample.
        let delayed = ElstAtom { entries: vec![edit(45000, -1), edit(918000, 3000)] };
        assert_eq!(delayed.first_media_time(90000, 90000), 3000 - 45000);

        // Empty edit spans 150 movie ticks at a movie timescale of 10000 and a media timescale
        // of 48000, i.e. 720 media ticks, and the media edit starts at media_time 0.
        let audio = ElstAtom { entries: vec![edit(150, -1), edit(85573, 0)] };
        assert_eq!(audio.first_media_time(10000, 48000), -720);
    }

    #[test]
    fn first_media_time_without_a_leading_empty_edit() {
        let media_time_only = ElstAtom { entries: vec![edit(918000, 1001)] };
        assert_eq!(media_time_only.first_media_time(90000, 90000), 1001);

        let empty_only = ElstAtom { entries: vec![edit(45000, -1)] };
        assert_eq!(empty_only.first_media_time(90000, 90000), 0);

        assert_eq!(ElstAtom { entries: vec![] }.first_media_time(90000, 90000), 0);
    }
}
