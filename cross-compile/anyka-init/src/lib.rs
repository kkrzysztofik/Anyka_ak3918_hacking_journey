//! Boot and process supervisor for the Anyka AK3918 SD-card hack.
//!
//! Entered from `/mnt/Factory/config.sh`, which the vendor's
//! `/usr/sbin/service.sh` runs when a `Factory/` directory is present on the
//! SD card. See `docs/plans/2026-08-01-boot-runtime-rust-design.md`.

pub mod config;
pub mod storm;
pub mod supervise;
pub mod sys;
pub mod timesync;
