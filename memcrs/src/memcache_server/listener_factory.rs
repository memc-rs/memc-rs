use socket2::{Domain, SockAddr, Socket, Type};
use std::net::{SocketAddr, ToSocketAddrs};

use crate::{
    memcache::cli::parser::MemcrsdConfig,
    memcache_server::{listen_socket_config::ListenSocketConfig, port_file_writer::PortFileWriter},
};

#[derive(Clone)]
pub struct ListenerFactory {
    config: ListenSocketConfig,
    factory: ListenerSocketFactory,
}

pub fn create_listener_from_config(memc_config: MemcrsdConfig) -> ListenerFactory {
    let config = ListenSocketConfig {
        port: memc_config.port,
        listen_address: memc_config.listen_address,
        listen_backlog: memc_config.backlog_limit,
    };
    let mut factory = ListenerSocketFactory { config };
    let listener_config = factory.determine_port();
    let port_file_writer = PortFileWriter::new();
    // ignoring results as all errors should be logged by PortFileWriter
    // and not writing port to a file should not block server start
    let _res = port_file_writer.write_port_to_file(listener_config);
    ListenerFactory {
        config: listener_config,
        factory,
    }
}

impl ListenerFactory {
    pub fn get_tcp_listener(&self) -> Result<std::net::TcpListener, std::io::Error> {
        let socket = self.factory.create_socket()?;
        let addr = SocketAddr::new(self.config.listen_address, self.config.port as u16);
        let addrs_iter = addr.to_socket_addrs()?;
        for socket_addr in addrs_iter {
            log::debug!("Binding to addr: {:?}", socket_addr);
            let sock_addr = SockAddr::from(socket_addr);
            let res = socket.bind(&sock_addr);
            if let Err(err) = res {
                log::error!("Can't bind to: {:?}, err {:?}", sock_addr, err);
                return Err(err);
            }
        }

        if let Err(err) = socket.listen(self.config.listen_backlog as i32) {
            log::error!("Listen error: {:?}", err);
            return Err(err);
        }

        let std_listener: std::net::TcpListener = socket.into();
        Ok(std_listener)
    }
}

#[derive(Clone, Copy)]
struct ListenerSocketFactory {
    config: ListenSocketConfig,
}

impl ListenerSocketFactory {
    fn determine_port(&mut self) -> ListenSocketConfig {
        if self.need_determine_port() {
            let socket = self.create_socket().unwrap_or_else(|err| {
                log::error!("Cannot determine port, socket creation failure: {:?}", err);
                std::process::exit(1);
            });
            let addr = SocketAddr::new(self.config.listen_address, 0);
            let socket_addr = socket2::SockAddr::from(addr);
            let res = socket.bind(&socket_addr);
            if let Err(err) = res {
                log::error!(
                    "Cannot determine port, bind syscall failure {:?}, err {:?}",
                    socket_addr,
                    err
                );
                std::process::exit(1);
            }
            match socket.local_addr() {
                Ok(addr) => match addr.as_socket() {
                    Some(resolved_addr) => {
                        self.config.port = resolved_addr.port() as i32;
                        log::info!("Determined port: {:?}", self.config.port);
                        return self.config;
                    }
                    None => {
                        log::error!(
                            "Cannot determine port, conversion to socket addr failure: {:?}",
                            socket
                        );
                    }
                },
                Err(err) => {
                    log::error!(
                        "Cannot determine port, cannot convert to local address {:?}, err {:?}",
                        socket_addr,
                        err
                    );
                    std::process::exit(1);
                }
            }
        }
        self.config
    }

    fn need_determine_port(&self) -> bool {
        self.config.port == -1
    }

    fn create_socket(&self) -> Result<socket2::Socket, std::io::Error> {
        let socket = Socket::new(Domain::IPV4, Type::STREAM, None)?;
        socket.set_reuse_address(true).unwrap_or_else(|err| {
            log::error!("Syscall to reuse address failure: {}", err);
            std::process::exit(1);
        });
        socket.set_reuse_port(true).unwrap_or_else(|err| {
            log::error!("Syscall to reuse address failure: {}", err);
            std::process::exit(1);
        });
        socket.set_nonblocking(true).unwrap_or_else(|err| {
            log::error!("Syscall to reuse address failure: {}", err);
            std::process::exit(1);
        });
        Ok(socket)
    }
}
