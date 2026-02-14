pub mod http_method_name {
    pub const OPTIONS: &str = "OPTIONS";
    pub const PATCH: &str = "PATCH";
    pub const POST: &str = "POST";
    pub const DELETE: &str = "DELETE";
    pub const GET: &str = "GET";
    pub const HEAD: &str = "HEAD";
    pub const PUT: &str = "PUT";
    pub const CONNECT: &str = "CONNECT";
    pub const TRACE: &str = "TRACE";
}

#[cfg(test)]
mod tests {
    use super::http_method_name;

    #[test]
    fn test_http_method_names() {
        assert_eq!(http_method_name::OPTIONS, "OPTIONS");
        assert_eq!(http_method_name::PATCH, "PATCH");
        assert_eq!(http_method_name::POST, "POST");
        assert_eq!(http_method_name::DELETE, "DELETE");
        assert_eq!(http_method_name::GET, "GET");
        assert_eq!(http_method_name::HEAD, "HEAD");
        assert_eq!(http_method_name::PUT, "PUT");
        assert_eq!(http_method_name::CONNECT, "CONNECT");
        assert_eq!(http_method_name::TRACE, "TRACE");
    }

    #[test]
    fn test_http_methods_are_uppercase() {
        let methods = [
            http_method_name::OPTIONS,
            http_method_name::PATCH,
            http_method_name::POST,
            http_method_name::DELETE,
            http_method_name::GET,
            http_method_name::HEAD,
            http_method_name::PUT,
            http_method_name::CONNECT,
            http_method_name::TRACE,
        ];
        for method in methods {
            assert_eq!(
                method,
                method.to_uppercase(),
                "Method {method} should be uppercase"
            );
        }
    }

    #[test]
    fn test_http_methods_are_distinct() {
        let methods = [
            http_method_name::OPTIONS,
            http_method_name::PATCH,
            http_method_name::POST,
            http_method_name::DELETE,
            http_method_name::GET,
            http_method_name::HEAD,
            http_method_name::PUT,
            http_method_name::CONNECT,
            http_method_name::TRACE,
        ];
        for i in 0..methods.len() {
            for j in (i + 1)..methods.len() {
                assert_ne!(methods[i], methods[j]);
            }
        }
    }
}
