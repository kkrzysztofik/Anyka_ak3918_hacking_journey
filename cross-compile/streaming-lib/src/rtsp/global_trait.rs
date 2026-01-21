pub trait Unmarshal {
    fn unmarshal(request_data: &str) -> Result<Self, String>
    where
        Self: Sized;
}

pub trait Marshal {
    fn marshal(&self) -> String;
}
