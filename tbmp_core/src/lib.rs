use crossbeam_channel::{self, Receiver, Sender};
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::vec::Vec;

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

pub trait Game : Serialize + DeserializeOwned + Send + Clone + 'static {

    type Move : Serialize + DeserializeOwned + Copy + Send;

    fn validate_move(&self, qmove: Self::Move) -> Result<(), ()>;
    fn apply_move(&mut self, qmove: Self::Move) -> MoveResult;
    fn default_board() -> Self;

    /// Accessors
    fn player_count(&self) -> u8;
    fn turn_of(&self) -> u8;
}

pub fn new_game<G: Game>() -> Vec<AgentCore<G>> {
    let mut game = G::default_board();

    let channels = (0..game.player_count()).fold(vec![], |mut tuples, _| {
        tuples.push((
            crossbeam_channel::unbounded::<G::Move>(),
            crossbeam_channel::unbounded::<GameEvent<G>>(),
        ));
        tuples
    });

    let (cores, anti_cores) = channels
        .into_iter()
        .fold((vec![], vec![]), |mut vecs, channels| {
            vecs.0.push(AgentCore {
                move_channel: (channels.0).0,
                event_channel: (channels.1).1,
            });
            vecs.1.push(AntiCore {
                move_channel: (channels.0).1,
                event_channel: (channels.1).0,
            });
            vecs
        });

    let mut main_thread = move || -> Result<(), Box<dyn std::error::Error>> {
        use GameEvent::*;

        for i in 0..game.player_count() {
            anti_cores[i as usize]
                .event_channel
                .send(GameStart(Clone::clone(&game), i))
                .unwrap();
        }

        anti_cores[game.turn_of() as usize]
            .event_channel
            .send(YourTurn)
            .unwrap();

        
            
        loop {
            let qmove = anti_cores[game.turn_of() as usize]
                .move_channel
                .recv()?;

            if let Ok(()) = G::validate_move(&game, qmove) {
                let current_player = game.turn_of();
                for i in 0..game.player_count()
                {
                    anti_cores[i as usize]
                        .event_channel
                        .send(MoveHappened(qmove))?;
                }
                match G::apply_move(&mut game, qmove) {
                    MoveResult::Continue => {}
                    MoveResult::Draw => {
                        for i in 0..game.player_count() {
                            anti_cores[i as usize]
                                .event_channel
                                .send(GameEnd(None))?;
                        }
                        break;
                    }
                    MoveResult::Win(side) => {
                        for i in 0..game.player_count() {
                            anti_cores[i as usize]
                                .event_channel
                                .send(GameEnd(Some(side)))?;
                        }
                        break;
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
                continue;
            }
        }
        
        Ok(())
    };

    std::thread::spawn(move ||
    {
        if let Err(e) = main_thread() {
            println!("{:?}", e);
        } else {

        }
    });

    cores
}
