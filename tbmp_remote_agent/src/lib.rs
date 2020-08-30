use tbmp_core::*;
use std::net::*;

pub fn connect<G: Game>(addr: SocketAddr) -> AgentCore<G> {
    let (tx, rx) = remote_channel::connect(addr).unwrap();
    AgentCore {
        move_channel: tx,
        event_channel: rx,
    }
}

pub fn host<G: Game>(core: AgentCore<G>, socket: u16) {
    remote_channel::offer_connection(core.move_channel, core.event_channel, socket).unwrap();
}
