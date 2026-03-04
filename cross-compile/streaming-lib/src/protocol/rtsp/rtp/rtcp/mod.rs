pub mod errors;
pub mod rtcp_app;
pub mod rtcp_bye;
pub mod rtcp_context;
pub mod rtcp_header;
pub mod rtcp_rr;
pub mod rtcp_sr;

pub const RTCP_SR: u8 = 200;
pub const RTCP_RR: u8 = 201;
pub const RTCP_SDES: u8 = 202;
pub const RTCP_BYE: u8 = 203;
pub const RTCP_APP: u8 = 204;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtcp_type_values() {
        assert_eq!(RTCP_SR, 200);
        assert_eq!(RTCP_RR, 201);
        assert_eq!(RTCP_SDES, 202);
        assert_eq!(RTCP_BYE, 203);
        assert_eq!(RTCP_APP, 204);
    }

    #[test]
    fn test_rtcp_types_are_contiguous() {
        assert_eq!(RTCP_RR, RTCP_SR + 1);
        assert_eq!(RTCP_SDES, RTCP_SR + 2);
        assert_eq!(RTCP_BYE, RTCP_SR + 3);
        assert_eq!(RTCP_APP, RTCP_SR + 4);
    }

    #[test]
    fn test_rtcp_types_are_distinct() {
        let types = [RTCP_SR, RTCP_RR, RTCP_SDES, RTCP_BYE, RTCP_APP];
        for i in 0..types.len() {
            for j in (i + 1)..types.len() {
                assert_ne!(types[i], types[j]);
            }
        }
    }
}
