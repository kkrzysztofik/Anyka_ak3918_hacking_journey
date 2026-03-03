use std::fmt;

pub mod rtsp_method_name {
    pub const OPTIONS: &str = "OPTIONS";
    pub const DESCRIBE: &str = "DESCRIBE";
    pub const ANNOUNCE: &str = "ANNOUNCE";
    pub const SETUP: &str = "SETUP";
    pub const PLAY: &str = "PLAY";
    pub const PAUSE: &str = "PAUSE";
    pub const TEARDOWN: &str = "TEARDOWN";
    pub const GET_PARAMETER: &str = "GET_PARAMETER";
    pub const SET_PARAMETER: &str = "SET_PARAMETER";
    pub const REDIRECT: &str = "REDIRECT";
    pub const RECORD: &str = "RECORD";

    pub const ARRAY: [&str; 11] = [
        OPTIONS,
        DESCRIBE,
        ANNOUNCE,
        SETUP,
        PLAY,
        PAUSE,
        TEARDOWN,
        GET_PARAMETER,
        SET_PARAMETER,
        REDIRECT,
        RECORD,
    ];

    /// Methods advertised in OPTIONS Public header (C→S only, excludes REDIRECT per RFC 2326 §10.10).
    pub const PUBLIC_METHODS: [&str; 10] = [
        OPTIONS,
        DESCRIBE,
        ANNOUNCE,
        SETUP,
        PLAY,
        PAUSE,
        TEARDOWN,
        GET_PARAMETER,
        SET_PARAMETER,
        RECORD,
    ];
}

pub enum ServerSessionType {
    Pull,
    Push,
}

impl fmt::Display for ServerSessionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let client_type = match self {
            ServerSessionType::Pull => "pull",
            ServerSessionType::Push => "push",
        };
        write!(f, "{client_type}")
    }
}

pub const USER_AGENT: &str = concat!("xiu ", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests {
    use super::*;

    // ========== RTSP Method Name Constants Tests ==========

    #[test]
    fn test_rtsp_method_options() {
        assert_eq!(rtsp_method_name::OPTIONS, "OPTIONS");
    }

    #[test]
    fn test_rtsp_method_describe() {
        assert_eq!(rtsp_method_name::DESCRIBE, "DESCRIBE");
    }

    #[test]
    fn test_rtsp_method_announce() {
        assert_eq!(rtsp_method_name::ANNOUNCE, "ANNOUNCE");
    }

    #[test]
    fn test_rtsp_method_setup() {
        assert_eq!(rtsp_method_name::SETUP, "SETUP");
    }

    #[test]
    fn test_rtsp_method_play() {
        assert_eq!(rtsp_method_name::PLAY, "PLAY");
    }

    #[test]
    fn test_rtsp_method_pause() {
        assert_eq!(rtsp_method_name::PAUSE, "PAUSE");
    }

    #[test]
    fn test_rtsp_method_teardown() {
        assert_eq!(rtsp_method_name::TEARDOWN, "TEARDOWN");
    }

    #[test]
    fn test_rtsp_method_get_parameter() {
        assert_eq!(rtsp_method_name::GET_PARAMETER, "GET_PARAMETER");
    }

    #[test]
    fn test_rtsp_method_set_parameter() {
        assert_eq!(rtsp_method_name::SET_PARAMETER, "SET_PARAMETER");
    }

    #[test]
    fn test_rtsp_method_redirect() {
        assert_eq!(rtsp_method_name::REDIRECT, "REDIRECT");
    }

    #[test]
    fn test_rtsp_method_record() {
        assert_eq!(rtsp_method_name::RECORD, "RECORD");
    }

    #[test]
    fn test_rtsp_method_array_length() {
        assert_eq!(rtsp_method_name::ARRAY.len(), 11);
    }

    #[test]
    fn test_rtsp_method_array_contains_all_methods() {
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::OPTIONS));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::DESCRIBE));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::ANNOUNCE));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::SETUP));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::PLAY));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::PAUSE));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::TEARDOWN));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::GET_PARAMETER));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::SET_PARAMETER));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::REDIRECT));
        assert!(rtsp_method_name::ARRAY.contains(&rtsp_method_name::RECORD));
    }

    // ========== ServerSessionType Display Tests ==========

    #[test]
    fn test_server_session_type_pull_display() {
        let session_type = ServerSessionType::Pull;
        assert_eq!(format!("{}", session_type), "pull");
    }

    #[test]
    fn test_server_session_type_push_display() {
        let session_type = ServerSessionType::Push;
        assert_eq!(format!("{}", session_type), "push");
    }

    // ========== PUBLIC_METHODS Tests ==========

    #[test]
    fn test_public_methods_excludes_redirect() {
        assert!(!rtsp_method_name::PUBLIC_METHODS.contains(&rtsp_method_name::REDIRECT));
    }

    #[test]
    fn test_public_methods_length() {
        assert_eq!(rtsp_method_name::PUBLIC_METHODS.len(), 10);
    }

    // ========== USER_AGENT Constant Test ==========

    #[test]
    fn test_user_agent_constant() {
        assert!(USER_AGENT.starts_with("xiu"));
        assert!(USER_AGENT.contains(env!("CARGO_PKG_VERSION")));
    }
}
