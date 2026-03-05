//! Unified stream pipeline telemetry.
//!
//! Combines queue-level statistics (enqueued, dropped, depth) with
//! fanout-level statistics (frames dispatched, bytes transferred, lag).

use std::sync::Arc;

use super::bridge::LowLatencyFrameQueue;

/// Unified telemetry for a single stream's frame pipeline.
///
/// Holds fanout-local counters (single-task, non-atomic) and a reference
/// to the queue for snapshotting push/drop statistics.
pub(crate) struct StreamTelemetry {
    stream_name: String,
    queue: Arc<LowLatencyFrameQueue>,

    pub frame_count: u64,
    pub total_bytes: u64,
    pub first_frame_logged: bool,
    lag_over_250ms: u64,
    lag_over_500ms: u64,
}

impl StreamTelemetry {
    pub fn new(stream_name: String, queue: Arc<LowLatencyFrameQueue>) -> Self {
        Self {
            stream_name,
            queue,
            frame_count: 0,
            total_bytes: 0,
            first_frame_logged: false,
            lag_over_250ms: 0,
            lag_over_500ms: 0,
        }
    }

    /// Record a dispatched frame's size.
    pub fn record_frame(&mut self, frame_size: u64) {
        self.frame_count += 1;
        self.total_bytes += frame_size;
    }

    /// Record queue-to-fanout lag threshold crossing.
    pub fn record_lag(&mut self, delay_ms: u64) {
        if delay_ms > 250 {
            self.lag_over_250ms += 1;
        }
        if delay_ms > 500 {
            self.lag_over_500ms += 1;
        }
    }

    /// Emit combined pipeline telemetry every 300 frames.
    ///
    /// Merges fanout counters with a snapshot of queue statistics
    /// into a single structured log entry.
    pub fn maybe_emit(&self) {
        if self.frame_count == 0 || !self.frame_count.is_multiple_of(300) {
            return;
        }

        let qs = self.queue.snapshot();
        tracing::debug!(
            stream = %self.stream_name,
            frames = self.frame_count,
            total_bytes = self.total_bytes,
            lag_over_250ms = self.lag_over_250ms,
            lag_over_500ms = self.lag_over_500ms,
            queue_capacity = self.queue.capacity(),
            queue_enqueued = qs.enqueued,
            queue_dropped_overflow = qs.dropped_on_overflow,
            queue_dropped_flush = qs.dropped_on_flush,
            queue_max_depth = qs.max_depth,
            "Stream pipeline telemetry"
        );
    }
}
