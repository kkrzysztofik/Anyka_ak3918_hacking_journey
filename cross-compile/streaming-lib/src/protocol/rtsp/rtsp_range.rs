use crate::protocol::rtsp::global_trait::Marshal;

use super::global_trait::Unmarshal;
use super::rtsp_utils;

#[derive(Debug, Clone, Default, PartialEq)]
pub enum RtspRangeType {
    #[default]
    NPT,
    CLOCK,
}

#[derive(Debug, Clone, Default)]
pub struct RtspRange {
    range_type: RtspRangeType,
    begin: i64,
    end: Option<i64>,
}

impl RtspRange {
    /// Parse a clock timestamp string into Unix timestamp.
    fn parse_clock_time(range_time: &str) -> Result<i64, String> {
        let datetime = chrono::NaiveDateTime::parse_from_str(range_time, "%Y%m%dT%H%M%SZ")
            .map_err(|err| {
                tracing::error!(error = %err, "get_clock_time_error");
                format!("invalid clock range: {range_time}")
            })?;
        Ok(datetime.and_utc().timestamp())
    }

    /// Parse clock range values from the value part.
    fn parse_clock_range(&mut self, value: &str) -> Result<(), String> {
        self.range_type = RtspRangeType::CLOCK;
        let ranges: Vec<&str> = value.split('-').collect();
        self.begin = Self::parse_clock_time(ranges[0])?;
        if ranges.len() > 1 && !ranges[1].is_empty() {
            self.end = Some(Self::parse_clock_time(ranges[1])?);
        }
        Ok(())
    }

    /// Parse NPT time in HH:MM:SS.mmm format.
    fn parse_npt_hhmmss(range_time: &str) -> Option<i64> {
        let (hour, minute, second, mill) =
            rtsp_utils::scanf!(range_time, |c| c == ':' || c == '.', i64, i64, i64, i64);
        if let (Some(hour), Some(minute), Some(second)) = (hour, minute, second) {
            let mut result = (hour * 3600 + minute * 60 + second) * 1000;
            if let Some(m) = mill {
                result += m;
            }
            return Some(result);
        }
        None
    }

    /// Parse NPT time in SS.mmm format (seconds with milliseconds).
    fn parse_npt_seconds_with_ms(range_time: &str) -> Option<i64> {
        let idx = range_time.find('.')?;
        let (sec_str, ms_str) = range_time.split_at(idx);
        let sec = sec_str.parse::<i64>().ok()?;
        let ms = ms_str[1..].parse::<i64>().ok()?;
        // Handle variable millisecond precision (e.g., .5 = 500ms, .500 = 500ms)
        let ms_len = ms_str.len() - 1; // exclude the dot
        let multiplier = match ms_len {
            1 => 100, // .5 -> 500
            2 => 10,  // .50 -> 500
            _ => 1,   // .500 -> 500
        };
        Some(sec * 1000 + ms * multiplier)
    }

    /// Parse NPT time value (supports HH:MM:SS.mmm, SS.mmm, and plain seconds).
    fn parse_npt_time(range_time: &str) -> Result<i64, String> {
        // Try HH:MM:SS.mmm format first
        if let Some(result) = Self::parse_npt_hhmmss(range_time) {
            return Ok(result);
        }

        // Try SS.mmm format (seconds with milliseconds)
        if let Some(result) = Self::parse_npt_seconds_with_ms(range_time) {
            return Ok(result);
        }

        // Try just seconds
        range_time
            .parse::<i64>()
            .map(|s| s * 1000)
            .map_err(|err| format!("invalid npt seconds '{range_time}': {err}"))
    }

    /// Parse NPT range values from the value part.
    fn parse_npt_range(&mut self, value: &str) -> Result<(), String> {
        self.range_type = RtspRangeType::NPT;
        let ranges: Vec<&str> = value.split('-').collect();

        self.begin = match ranges[0] {
            "now" => 0,
            _ => Self::parse_npt_time(ranges[0])?,
        };

        if ranges.len() == 2 && !ranges[1].is_empty() {
            self.end = Some(Self::parse_npt_time(ranges[1])?);
        }
        Ok(())
    }
}

impl Unmarshal for RtspRange {
    fn unmarshal(raw_data: &str) -> Result<Self, String> {
        let mut rtsp_range = RtspRange::default();

        let kv: Vec<&str> = raw_data.splitn(2, '=').collect();
        if kv.len() < 2 {
            return Err("invalid rtsp range format".to_string());
        }

        match kv[0] {
            "clock" => rtsp_range.parse_clock_range(kv[1])?,
            "npt" => rtsp_range.parse_npt_range(kv[1])?,
            _ => return Err(format!("unsupported range type: {}", kv[0])),
        }

        Ok(rtsp_range)
    }
}

impl Marshal for RtspRange {
    fn marshal(&self) -> String {
        match self.range_type {
            RtspRangeType::NPT => {
                let format_npt = |ms: i64| {
                    let clamped = ms.max(0);
                    let seconds = clamped / 1000;
                    let millis = (clamped % 1000).abs();
                    if millis == 0 {
                        format!("{}", seconds)
                    } else {
                        format!("{}.{:03}", seconds, millis)
                    }
                };

                let begin = format_npt(self.begin);
                let end = self.end.map(format_npt).unwrap_or_default();
                if end.is_empty() {
                    format!("npt={begin}-")
                } else {
                    format!("npt={begin}-{end}")
                }
            }
            RtspRangeType::CLOCK => {
                let format_clock = |timestamp: i64| {
                    chrono::DateTime::from_timestamp(timestamp, 0)
                        .map(|dt| dt.format("%Y%m%dT%H%M%SZ").to_string())
                        .unwrap_or("19700101T000000Z".to_string())
                };

                let begin = format_clock(self.begin);
                if let Some(end) = self.end {
                    let end_str = format_clock(end);
                    format!("clock={begin}-{end_str}")
                } else {
                    format!("clock={begin}-")
                }
            }
        }
    }
}

impl RtspRange {
    pub fn range_type(&self) -> RtspRangeType {
        self.range_type.clone()
    }

    pub fn begin(&self) -> i64 {
        self.begin
    }

    pub fn end(&self) -> Option<i64> {
        self.end
    }
}

#[cfg(test)]
mod tests {

    use super::{RtspRange, RtspRangeType};
    use crate::protocol::rtsp::global_trait::Marshal;
    use crate::protocol::rtsp::global_trait::Unmarshal;

    #[test]
    fn test_parse_transport() {
        //a=range:
        //a=range:npt=now-
        //a=range:npt=0-
        let parser = RtspRange::unmarshal("clock=20220520T064812Z-20230520T064816Z").unwrap();

        assert_eq!(parser.range_type(), RtspRangeType::CLOCK);
        assert!(parser.begin() > 0);
        assert!(parser.end().is_some());

        let parser1 = RtspRange::unmarshal("npt=now-").unwrap();
        assert_eq!(parser1.range_type(), RtspRangeType::NPT);
        assert_eq!(parser1.begin(), 0);
        assert!(parser1.end().is_none());

        let parser2 = RtspRange::unmarshal("npt=0-").unwrap();
        assert_eq!(parser2.range_type(), RtspRangeType::NPT);
        assert_eq!(parser2.begin(), 0);
        assert!(parser2.end().is_none());
    }

    // ============================================
    // NPT Range Tests
    // ============================================

    #[test]
    fn test_unmarshal_npt_now() {
        let range = RtspRange::unmarshal("npt=now-").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        assert_eq!(range.begin(), 0);
        assert!(range.end().is_none());
    }

    #[test]
    fn test_unmarshal_npt_zero() {
        let range = RtspRange::unmarshal("npt=0-").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        assert_eq!(range.begin(), 0);
        assert!(range.end().is_none());
    }

    #[test]
    fn test_unmarshal_npt_with_end() {
        let range = RtspRange::unmarshal("npt=10.5-20.5").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        // begin = 10 seconds + 500ms = 10500ms
        assert_eq!(range.begin(), 10500);
        assert_eq!(range.end(), Some(20500));
    }

    #[test]
    fn test_unmarshal_npt_hours_minutes_seconds() {
        let range = RtspRange::unmarshal("npt=1:2:3.500-2:3:4.600").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        // 1:2:3.500 = 1*3600 + 2*60 + 3 = 3723 seconds + 500ms = 3723500ms
        assert_eq!(range.begin(), 3723500);
        // 2:3:4.600 = 2*3600 + 3*60 + 4 = 7384 seconds + 600ms = 7384600ms
        assert_eq!(range.end(), Some(7384600));
    }

    #[test]
    fn test_unmarshal_npt_seconds_only() {
        let range = RtspRange::unmarshal("npt=123.456-456.789").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        // 123 seconds + 456ms = 123456ms
        assert_eq!(range.begin(), 123456);
        // 456 seconds + 789ms = 456789ms
        assert_eq!(range.end(), Some(456789));
    }

    #[test]
    fn test_unmarshal_npt_no_milliseconds() {
        let range = RtspRange::unmarshal("npt=10-20").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        // 10 seconds = 10000ms
        assert_eq!(range.begin(), 10000);
        assert_eq!(range.end(), Some(20000));
    }

    // ============================================
    // Marshal Tests
    // ============================================

    #[test]
    fn test_marshal_npt_range() {
        let range = RtspRange::unmarshal("npt=10.5-20.25").unwrap();
        assert_eq!(range.marshal(), "npt=10.500-20.250");
    }

    #[test]
    fn test_marshal_clock_range() {
        let range = RtspRange::unmarshal("clock=20220520T064812Z-20230520T064816Z").unwrap();
        assert_eq!(range.marshal(), "clock=20220520T064812Z-20230520T064816Z");
    }

    // ============================================
    // Clock Range Tests
    // ============================================

    #[test]
    fn test_unmarshal_clock_with_end() {
        let range = RtspRange::unmarshal("clock=20220520T064812Z-20230520T064816Z").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::CLOCK);
        assert!(range.begin() > 0);
        assert!(range.end().is_some());
        assert!(range.end().unwrap() > range.begin());
    }

    #[test]
    fn test_unmarshal_clock_no_end() {
        let range = RtspRange::unmarshal("clock=20220520T064812Z-").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::CLOCK);
        assert!(range.begin() > 0);
        assert!(range.end().is_none());
    }

    #[test]
    fn test_unmarshal_clock_single_timestamp() {
        let range = RtspRange::unmarshal("clock=20220520T064812Z").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::CLOCK);
        assert!(range.begin() > 0);
        assert!(range.end().is_none());
    }

    // ============================================
    // Error Handling Tests
    // ============================================

    #[test]
    fn test_unmarshal_invalid_format() {
        let range = RtspRange::unmarshal("invalid");
        assert!(range.is_err());
    }

    #[test]
    fn test_unmarshal_empty_string() {
        let range = RtspRange::unmarshal("");
        assert!(range.is_err());
    }

    #[test]
    fn test_unmarshal_unknown_type() {
        let range = RtspRange::unmarshal("unknown=value");
        assert!(range.is_err());
    }

    #[test]
    fn test_unmarshal_malformed_clock() {
        let range = RtspRange::unmarshal("clock=invalid-format");
        assert!(range.is_err());
    }

    // ============================================
    // Round-trip Tests
    // ============================================

    #[test]
    fn test_rtsp_range_npt_roundtrip() {
        let original_str = "npt=10.5-20.5";
        let range = RtspRange::unmarshal(original_str).unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        assert_eq!(range.begin(), 10500);
        assert_eq!(range.end(), Some(20500));
    }

    #[test]
    fn test_rtsp_range_clock_roundtrip() {
        let original_str = "clock=20220520T064812Z-20230520T064816Z";
        let range = RtspRange::unmarshal(original_str).unwrap();
        assert_eq!(range.range_type(), RtspRangeType::CLOCK);
        assert!(range.begin() > 0);
        assert!(range.end().is_some());
    }

    // ============================================
    // Edge Cases
    // ============================================

    #[test]
    fn test_unmarshal_npt_very_large() {
        let range = RtspRange::unmarshal("npt=999999:59:59.999-").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        // Should handle large values
        assert!(range.begin() >= 0);
    }

    #[test]
    fn test_unmarshal_npt_zero_milliseconds() {
        let range = RtspRange::unmarshal("npt=10.0-20.0").unwrap();
        assert_eq!(range.range_type(), RtspRangeType::NPT);
        assert_eq!(range.begin(), 10000);
        assert_eq!(range.end(), Some(20000));
    }
}
