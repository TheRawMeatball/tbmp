use crossbeam_channel;
use crossbeam_channel::{Receiver, Sender};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    error::Error,
    io::prelude::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
};

pub fn connect<A, B>(
    target: SocketAddr,
) -> Result<
    (
        Sender<B>,
        Receiver<A>,
        impl FnMut() -> Result<(), Box<dyn Error>>,
    ),
    Box<dyn Error>,
>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let stream = TcpStream::connect(target)?;
    let (tx1, rx1) = crossbeam_channel::unbounded::<B>();
    let (tx2, rx2) = crossbeam_channel::unbounded::<A>();

    Ok((tx1, rx2, connection_handler(rx1, tx2, stream)))
}

pub fn connect_direct<A, B>(
    tx: Sender<A>,
    rx: Receiver<B>,
    target: SocketAddr,
) -> Result<impl FnMut() -> Result<(), Box<dyn Error>>, Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let stream = TcpStream::connect(target)?;
    Ok(connection_handler(rx, tx, stream))
}

pub fn accept_connection<A, B>(
    port: u16,
) -> Result<
    (
        Sender<A>,
        Receiver<B>,
        impl FnMut() -> Result<(), Box<dyn Error>>,
    ),
    Box<dyn Error>,
>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let listener = TcpListener::bind(SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port))?;
    let (stream, _) = listener.accept()?;
    std::mem::drop(listener);

    let (tx1, rx1) = crossbeam_channel::unbounded::<A>();
    let (tx2, rx2) = crossbeam_channel::unbounded::<B>();

    Ok((tx1, rx2, connection_handler(rx1, tx2, stream)))
}

pub fn offer_connections<A, B>(
    channels: Vec<(Sender<B>, Receiver<A>)>,
    port: u16,
) -> Result<Vec<impl FnMut() -> Result<(), Box<dyn Error>>>, Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    let listener = TcpListener::bind(SocketAddr::new(Ipv4Addr::new(0, 0, 0, 0).into(), port))?;
    Ok(channels
        .into_iter()
        .map(|(tx, rx)| {
            let (stream, _) = listener.accept().unwrap();
            connection_handler(rx, tx, stream)
        })
        .collect())
}

fn connection_handler<A, B>(
    rx1: Receiver<A>,
    tx2: Sender<B>,
    mut stream: TcpStream,
) -> impl FnMut() -> Result<(), Box<dyn Error>>
where
    A: Send + Serialize + DeserializeOwned + 'static,
    B: Send + Serialize + DeserializeOwned + 'static,
{
    stream.set_nonblocking(true).unwrap();

    move || {
        stream.set_nonblocking(false).unwrap();
        while let Some(msg) = rx1.try_iter().next() {
            let bytes = bincode::serialize(&msg)?;
            stream.write(&bytes)?;
        }

        stream.set_nonblocking(true).unwrap();
        let mut buf = [0u8; 1024];
        if let Ok(size) = stream.read(&mut buf) {
            let mut processed = 0;
            loop {
                let msg  = bincode::deserialize(&buf[processed..size])?;
                processed += bincode::serialized_size(&msg)? as usize;
                tx2.send(msg)?;
                if processed == size {
                    break;
                }
            }
        }

        Ok(())
    }
}
