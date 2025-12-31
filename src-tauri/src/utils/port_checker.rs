use std::net::TcpListener;

/// 检查指定端口是否可用
pub fn is_port_available(port: u16) -> bool {
    TcpListener::bind(("127.0.0.1", port)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find_next_available_port(start: u16, end: u16) -> Option<u16> {
        for port in start..=end {
            if is_port_available(port) {
                return Some(port);
            }
        }
        None
    }

    #[test]
    fn test_is_port_available() {
        // 测试一个通常不会被占用的高端口
        let port = 59999;
        assert!(is_port_available(port));
    }

    #[test]
    fn test_find_next_available_port() {
        let result = find_next_available_port(58000, 59000);
        assert!(result.is_some());
    }
}
