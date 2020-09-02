use std::error::Error;
use std::net::*;
use tbmp_core::*;

pub fn connect<G: Game>(
    addr: SocketAddr,
) -> (AgentCore<G>, impl FnMut() -> Result<(), Box<dyn Error>>) {
    let (tx, rx, thread) = remote_channel::connect(addr).unwrap();
    (
        AgentCore {
            move_channel: tx,
            event_channel: rx,
        },
        thread,
    )
}

pub fn host<G: Game>(
    cores: Vec<AgentCore<G>>,
    socket: u16,
) -> Vec<impl FnMut() -> Result<(), Box<dyn Error>>> {
    remote_channel::offer_connections(
        cores
            .into_iter()
            .map(|core| (core.move_channel, core.event_channel))
            .collect(),
        socket,
    )
    .unwrap()
}
