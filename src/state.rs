use zbus::Connection;
use crate::config::Config;

pub struct XdpwState {
    pub config: Config,
    pub connection: Connection,
}

