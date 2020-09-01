use std::net::*;
use std::error::Error;
use tbmp_core::*;

pub fn connect<G: Game>(addr: SocketAddr) -> (AgentCore<G>, impl FnMut() -> Result<(), Box<dyn Error>>) {
    let (tx, rx, thread) = remote_channel::connect(addr).unwrap();
    (
        AgentCore {
            move_channel: tx,
            event_channel: rx,
        },
        thread,
    )
}

pub fn host<G: Game>(core: AgentCore<G>, socket: u16) -> impl FnMut() -> Result<(), Box<dyn Error>> {
    remote_channel::offer_connection(core.move_channel, core.event_channel, socket).unwrap()
}
