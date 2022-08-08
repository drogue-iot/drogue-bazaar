#[inline]
pub fn max_payload_size() -> usize {
    65536
}

#[inline]
pub fn max_json_payload_size() -> usize {
    65536
}

#[inline]
pub fn bind_addr() -> String {
    "[::1]:8080".into()
}
