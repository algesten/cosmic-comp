// SPDX-License-Identifier: GPL-3.0-only

//! Tile-merge support for DisplayID-tiled monitors.
//!
//! A monitor that exposes itself over multiple DisplayPort streams (LG
//! UltraFine 5K, Apple Studio Display, Dell UP2715K, …) advertises a DRM
//! `TILE` blob on each member connector. Connectors that share a tile group
//! collapse into one logical [`smithay::output::Output`] of size
//! `(sum tile_w, max tile_h)` backed by N CRTCs, each scanning out from a
//! sub-rect of the logical framebuffer.
//!
//! This mirrors Mutter's `MetaMonitorTiled` design: each tile remains an
//! ordinary CRTC with its own framebuffer; the merge happens at the
//! logical-monitor abstraction, not in the renderer.

use smithay::{
    backend::drm::TileInfo,
    reexports::drm::control::{connector, crtc},
    utils::{Physical, Point, Rectangle, Size},
};

/// One CRTC's placement within a merged tile group.
///
/// Gathered as the group's tile connectors are enumerated, then handed (all of
/// a group at once) to [`smithay::backend::drm::output::DrmOutputManager::initialize_tiled_output`]
/// to build a single frame-locked compositor that drives every member CRTC. The
/// per-CRTC slicing lives in smithay's `new_tiled`, not here — cosmic-comp does
/// not relocate/slice elements itself.
#[derive(Debug, Clone)]
pub struct TileSlot {
    /// The CRTC scanning out this tile.
    pub crtc: crtc::Handle,
    /// The connector for this tile.
    pub connector: connector::Handle,
    /// Sub-rect of the merged logical output this CRTC scans out, in the
    /// output's untransformed coordinate space (`(loc·tile_size, tile_size)`).
    pub region: Rectangle<i32, Physical>,
    /// The `(0, 0)` tile — its CRTC keys the group's surface and its vblank
    /// drives presentation for the whole group.
    pub primary: bool,
}

impl TileSlot {
    /// Build a slot for `crtc`/`connector` from its [`TileInfo`].
    pub fn from_tile_info(crtc: crtc::Handle, connector: connector::Handle, info: &TileInfo) -> Self {
        let tile_w = i32::from(info.tile_w);
        let tile_h = i32::from(info.tile_h);
        let loc_h = i32::from(info.loc_h);
        let loc_v = i32::from(info.loc_v);
        Self {
            crtc,
            connector,
            region: Rectangle::new(
                Point::from((tile_w * loc_h, tile_h * loc_v)),
                Size::from((tile_w, tile_h)),
            ),
            primary: loc_h == 0 && loc_v == 0,
        }
    }

    /// Number of tiles in `info`'s group (`num_h × num_v`) — how many slots to
    /// gather before the group is complete.
    pub fn expected_count(info: &TileInfo) -> usize {
        usize::from(info.num_h_tiles) * usize::from(info.num_v_tiles)
    }
}
