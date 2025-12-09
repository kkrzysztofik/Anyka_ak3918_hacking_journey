//! Integration tests for XML namespace parsing flexibility.
//!
//! These tests verify that ONVIF types can be deserialized correctly
//! whether they use the tt: prefix or unprefixed field names.
//! This is critical for ODM (ONVIF Device Manager) compatibility.

use onvif_rust::onvif::types::common::{Date, DateTime, Time, TimeZone};

// ============================================================================
// DateTime Namespace Parsing Tests
// ============================================================================

/// Test that DateTime can parse with tt: prefixed fields (standard ONVIF)
#[test]
fn test_datetime_with_tt_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <tt:DateTime>
            <tt:Time>
                <tt:Hour>14</tt:Hour>
                <tt:Minute>30</tt:Minute>
                <tt:Second>45</tt:Second>
            </tt:Time>
            <tt:Date>
                <tt:Year>2024</tt:Year>
                <tt:Month>12</tt:Month>
                <tt:Day>2</tt:Day>
            </tt:Date>
        </tt:DateTime>"#;

    let datetime: DateTime = quick_xml::de::from_str(xml).expect("Should parse tt: prefixed XML");

    assert_eq!(datetime.time.hour, 14);
    assert_eq!(datetime.time.minute, 30);
    assert_eq!(datetime.time.second, 45);
    assert_eq!(datetime.date.year, 2024);
    assert_eq!(datetime.date.month, 12);
    assert_eq!(datetime.date.day, 2);
}

/// Test that DateTime can parse without tt: prefix (ODM compatibility)
#[test]
fn test_datetime_without_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <DateTime>
            <Time>
                <Hour>10</Hour>
                <Minute>15</Minute>
                <Second>30</Second>
            </Time>
            <Date>
                <Year>2024</Year>
                <Month>6</Month>
                <Day>15</Day>
            </Date>
        </DateTime>"#;

    let datetime: DateTime = quick_xml::de::from_str(xml).expect("Should parse unprefixed XML");

    assert_eq!(datetime.time.hour, 10);
    assert_eq!(datetime.time.minute, 15);
    assert_eq!(datetime.time.second, 30);
    assert_eq!(datetime.date.year, 2024);
    assert_eq!(datetime.date.month, 6);
    assert_eq!(datetime.date.day, 15);
}

// ============================================================================
// Time Namespace Parsing Tests
// ============================================================================

/// Test Time parsing with tt: prefix
#[test]
fn test_time_with_tt_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <tt:Time>
            <tt:Hour>23</tt:Hour>
            <tt:Minute>59</tt:Minute>
            <tt:Second>59</tt:Second>
        </tt:Time>"#;

    let time: Time = quick_xml::de::from_str(xml).expect("Should parse tt: prefixed Time");

    assert_eq!(time.hour, 23);
    assert_eq!(time.minute, 59);
    assert_eq!(time.second, 59);
}

/// Test Time parsing without prefix
#[test]
fn test_time_without_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <Time>
            <Hour>0</Hour>
            <Minute>0</Minute>
            <Second>0</Second>
        </Time>"#;

    let time: Time = quick_xml::de::from_str(xml).expect("Should parse unprefixed Time");

    assert_eq!(time.hour, 0);
    assert_eq!(time.minute, 0);
    assert_eq!(time.second, 0);
}

// ============================================================================
// Date Namespace Parsing Tests
// ============================================================================

/// Test Date parsing with tt: prefix
#[test]
fn test_date_with_tt_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <tt:Date>
            <tt:Year>2025</tt:Year>
            <tt:Month>1</tt:Month>
            <tt:Day>1</tt:Day>
        </tt:Date>"#;

    let date: Date = quick_xml::de::from_str(xml).expect("Should parse tt: prefixed Date");

    assert_eq!(date.year, 2025);
    assert_eq!(date.month, 1);
    assert_eq!(date.day, 1);
}

/// Test Date parsing without prefix
#[test]
fn test_date_without_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <Date>
            <Year>2023</Year>
            <Month>12</Month>
            <Day>31</Day>
        </Date>"#;

    let date: Date = quick_xml::de::from_str(xml).expect("Should parse unprefixed Date");

    assert_eq!(date.year, 2023);
    assert_eq!(date.month, 12);
    assert_eq!(date.day, 31);
}

// ============================================================================
// TimeZone Namespace Parsing Tests
// ============================================================================

/// Test TimeZone parsing with tt: prefix
#[test]
fn test_timezone_with_tt_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <tt:TimeZone>
            <tt:TZ>UTC+01:00</tt:TZ>
        </tt:TimeZone>"#;

    let tz: TimeZone = quick_xml::de::from_str(xml).expect("Should parse tt: prefixed TimeZone");

    assert_eq!(tz.tz, "UTC+01:00");
}

/// Test TimeZone parsing without prefix (critical for ODM SetSystemDateAndTime)
#[test]
fn test_timezone_without_prefix() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <TimeZone>
            <TZ>America/New_York</TZ>
        </TimeZone>"#;

    let tz: TimeZone = quick_xml::de::from_str(xml).expect("Should parse unprefixed TimeZone");

    assert_eq!(tz.tz, "America/New_York");
}

// ============================================================================
// Mixed Prefix Tests (Real-world scenarios)
// ============================================================================

/// Test parsing with mixed prefixes (some tools send inconsistent XML)
#[test]
fn test_datetime_mixed_prefix_time_prefixed() {
    // Time has prefix, Date doesn't
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <DateTime>
            <tt:Time>
                <tt:Hour>12</tt:Hour>
                <tt:Minute>30</tt:Minute>
                <tt:Second>0</tt:Second>
            </tt:Time>
            <Date>
                <Year>2024</Year>
                <Month>6</Month>
                <Day>1</Day>
            </Date>
        </DateTime>"#;

    let datetime: DateTime = quick_xml::de::from_str(xml).expect("Should parse mixed prefix XML");

    assert_eq!(datetime.time.hour, 12);
    assert_eq!(datetime.date.year, 2024);
}

/// Test the exact XML structure ODM sends for SetSystemDateAndTime
#[test]
fn test_odm_set_system_date_time_format() {
    // This is similar to what ODM sends
    let xml = r#"<?xml version="1.0" encoding="utf-8"?>
        <TimeZone xmlns:tt="http://www.onvif.org/ver10/schema">
            <TZ>Europe/London</TZ>
        </TimeZone>"#;

    let tz: TimeZone = quick_xml::de::from_str(xml).expect("Should parse ODM-style TimeZone");

    assert_eq!(tz.tz, "Europe/London");
}

// ============================================================================
// Roundtrip Tests (Serialize then Deserialize)
// ============================================================================

/// Test that DateTime survives a serialize/deserialize roundtrip
#[test]
fn test_datetime_roundtrip() {
    let original = DateTime {
        time: Time {
            hour: 15,
            minute: 45,
            second: 30,
        },
        date: Date {
            year: 2024,
            month: 7,
            day: 4,
        },
    };

    let xml = quick_xml::se::to_string(&original).expect("Should serialize DateTime");
    let parsed: DateTime = quick_xml::de::from_str(&xml).expect("Should deserialize DateTime");

    assert_eq!(parsed.time.hour, original.time.hour);
    assert_eq!(parsed.time.minute, original.time.minute);
    assert_eq!(parsed.time.second, original.time.second);
    assert_eq!(parsed.date.year, original.date.year);
    assert_eq!(parsed.date.month, original.date.month);
    assert_eq!(parsed.date.day, original.date.day);
}

/// Test that TimeZone survives a roundtrip
#[test]
fn test_timezone_roundtrip() {
    let original = TimeZone {
        tz: "America/Los_Angeles".to_string(),
    };

    let xml = quick_xml::se::to_string(&original).expect("Should serialize TimeZone");
    let parsed: TimeZone = quick_xml::de::from_str(&xml).expect("Should deserialize TimeZone");

    assert_eq!(parsed.tz, original.tz);
}
