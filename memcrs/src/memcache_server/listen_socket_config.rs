use std::net::IpAddr;

#[derive(Clone, Copy)]
pub struct ListenSocketConfig {
    pub listen_backlog: u32,
    pub listen_address: IpAddr,
    pub port: i32,
}
