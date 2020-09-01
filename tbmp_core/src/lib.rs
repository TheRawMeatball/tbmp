use crossbeam_channel::{self, Receiver, Sender};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{error::Error, vec::Vec};

#[derive(Clone)]
pub struct AgentCore<G: Game> {
    pub move_channel: Sender<G::Move>,
    pub event_channel: Receiver<GameEvent<G>>,
}

struct AntiCore<G: Game> {
    pub move_channel: Receiver<G::Move>,
    pub event_channel: Sender<GameEvent<G>>,
}

#[derive(Serialize, Deserialize)]
pub enum GameEvent<G: Game> {
    #[serde(bound = "")]
    GameStart(G, PlayerID),
    MoveHappened(G::Move),
    YourTurn,
    ValidMove,
    InvalidMove,
    OpponentQuit,
    GameEnd(Option<PlayerID>),
}

pub type PlayerID = u8;

pub enum MoveResult {
    Continue,
    Draw,
    Win(PlayerID),
}

pub trait Game: Serialize + DeserializeOwned + Send + Clone + 'static {
    type Move: Serialize + DeserializeOwned + Copy + Send;

    const PLAYER_COUNT: u8;

    fn validate_move(&self, qmove: Self::Move) -> Result<(), ()>;
    fn apply_move(&mut self, qmove: Self::Move) -> MoveResult;

    fn initial_server() -> Self;
    fn initial_for_player(&self, _: PlayerID) -> Self {
        Clone::clone(self)
    }

    // Accessor
    fn turn_of(&self) -> u8;
}

pub fn new_game<G: Game>() -> (
    Vec<AgentCore<G>>,
    impl FnMut() -> Result<(), Box<dyn Error>>,
) {
    let mut game = G::initial_server();

    let (cores, mut anti_cores) = (0..G::PLAYER_COUNT)
        .map(|_| {
            (
                crossbeam_channel::unbounded::<G::Move>(),
                crossbeam_channel::unbounded::<GameEvent<G>>(),
            )
        })
        .map(|channels| {
            (
                AgentCore {
                    move_channel: (channels.0).0,
                    event_channel: (channels.1).1,
                },
                AntiCore {
                    move_channel: (channels.0).1,
                    event_channel: (channels.1).0,
                },
            )
        })
        .unzip::<_, AntiCore<_>, Vec<_>, Vec<_>>();

    anti_cores
        .iter_mut()
        .map(|x| &x.event_channel)
        .enumerate()
        .for_each(|(i, x)| {
            x.send(GameEvent::GameStart(
                game.initial_for_player(i as u8),
                i as u8,
            ))
            .unwrap()
        });

    anti_cores[game.turn_of() as usize]
        .event_channel
        .send(GameEvent::YourTurn)
        .unwrap();

    let main_loop = move || -> Result<(), Box<dyn Error>> {
        use GameEvent::*;
        if let Ok(qmove) = anti_cores[game.turn_of() as usize].move_channel.try_recv() {
            if let Ok(()) = G::validate_move(&game, qmove) {
                let current_player = game.turn_of();
                for i in 0..G::PLAYER_COUNT {
                    anti_cores[i as usize]
                        .event_channel
                        .send(MoveHappened(qmove))?;
                }
                match G::apply_move(&mut game, qmove) {
                    MoveResult::Continue => {}
                    MoveResult::Draw => {
                        for i in 0..G::PLAYER_COUNT {
                            anti_cores[i as usize].event_channel.send(GameEnd(None))?;
                        }
                        return Ok(());
                    }
                    MoveResult::Win(side) => {
                        for i in 0..G::PLAYER_COUNT {
                            anti_cores[i as usize]
                                .event_channel
                                .send(GameEnd(Some(side)))?;
                        }
                        return Ok(());
                    }
                }

                anti_cores[current_player as usize]
                    .event_channel
                    .send(ValidMove)?;
                anti_cores[game.turn_of() as usize]
                    .event_channel
                    .send(YourTurn)?;
            } else {
                anti_cores[game.turn_of() as usize]
                    .event_channel
                    .send(InvalidMove)?;
            }
        }

        Ok(())
    };

    (cores, main_loop)
}
