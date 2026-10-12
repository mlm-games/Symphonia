// Symphonia
// Copyright (c)  2019-2026 The Project Symphonia Developers.
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use symphonia_core::codecs::CodecProfile;
use symphonia_core::codecs::video::VideoExtraData;
use symphonia_core::codecs::video::well_known::extra_data::VIDEO_EXTRA_DATA_ID_VP9_DECODER_CONFIG;

use crate::atoms::stsd::VisualSampleEntry;
use crate::atoms::{Atom, AtomHeader, AtomIterator, ReadAtom, Result, decode_error};

/// `VPCodecConfigurationBox` from the VP Codec ISO Media File Format Binding
/// spec. Payload is `FullBox` version/flags, profile, level, then a packed
/// byte of bit depth, chroma subsampling and full range flag, then the CICP
/// colour triple and the codec initialization data size.
#[derive(Debug)]
pub struct VpcCAtom {
    extra_data: VideoExtraData,
    profile: CodecProfile,
    level: u32,
}

impl Atom for VpcCAtom {
    fn read<R: ReadAtom>(it: &mut AtomIterator<R>, header: &AtomHeader) -> Result<Self> {
        let len = match header.data_size() {
            Some(len) => len as usize,
            None => {
                return decode_error("isomp4 (vpcC): expected atom size to be known");
            }
        };

        let data = it.read_boxed_slice_exact(len)?;

        if data.len() < 12 {
            return decode_error("isomp4 (vpcC): atom is too small");
        }

        let profile = CodecProfile::new(u32::from(data[4]));
        let level = u32::from(data[5]);

        Ok(Self {
            extra_data: VideoExtraData { id: VIDEO_EXTRA_DATA_ID_VP9_DECODER_CONFIG, data },
            profile,
            level,
        })
    }
}

impl VpcCAtom {
    pub fn fill_video_sample_entry(self, entry: &mut VisualSampleEntry) {
        entry.profile = Some(self.profile);
        entry.level = Some(self.level);
        entry.extra_data.push(self.extra_data);
    }
}
