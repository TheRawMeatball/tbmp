use crossbeam_channel;
use crossbeam_channel::{Receiver, Sender};
use serde::{Serialize, de::DeserializeOwned};
use std::error::Error;
use std::io::prelude::*;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::thread;

pub fn connect<A, B>(target: SocketAddr) -> Result<(Sender<B>, Receiver<A>), Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let stream = TcpStream::connect(target)?;
    let (tx1, rx1) = crossbeam_channel::unbounded::<B>();
    let (tx2, rx2) = crossbeam_channel::unbounded::<A>();

    thread::spawn(move || {
        handle_connection(rx1, tx2, stream);
    });

    Ok((tx1, rx2))
}

pub fn connect_direct<A, B>(
    tx: Sender<A>,
    rx: Receiver<B>,
    target: SocketAddr,
) -> Result<(), Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let stream = TcpStream::connect(target)?;
    thread::spawn(move || {
        handle_connection(rx, tx, stream);
    });
    Ok(())
}

pub fn accept_connection<A, B>(port: u16) -> Result<(Sender<A>, Receiver<B>), Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let listener = TcpListener::bind(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))?;
    let (stream, _) = listener.accept()?;
    std::mem::drop(listener);

    let (tx1, rx1) = crossbeam_channel::unbounded::<A>();
    let (tx2, rx2) = crossbeam_channel::unbounded::<B>();

    thread::spawn(move || {
        handle_connection(rx1, tx2, stream);
    });

    Ok((tx1, rx2))
}

pub fn offer_connection<A, B>(
    tx: Sender<B>,
    rx: Receiver<A>,
    port: u16,
) -> Result<(), Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let listener = TcpListener::bind(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))?;
    let (stream, _) = listener.accept()?;
    std::mem::drop(listener);

    thread::spawn(move || {
        handle_connection(rx, tx, stream);
    });

    Ok(())
}

fn handle_connection<A, B>(rx1: Receiver<A>, tx2: Sender<B>, mut stream: TcpStream) -> !
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    stream.set_nonblocking(true).unwrap();

    loop {
        if let Ok(msg) = rx1.try_recv() {
            let bytes = bincode::serialize(&msg).unwrap();
            stream.write(&bytes).expect("write to stream");
        }

        let mut buf = [0u8; 1024];
        if let Ok(_) = stream.read(&mut buf) {
            let msg = bincode::deserialize(&buf).expect("deserialize from stream");
            tx2.send(msg).expect("forward from stream to tx2");
        }
    }
}
