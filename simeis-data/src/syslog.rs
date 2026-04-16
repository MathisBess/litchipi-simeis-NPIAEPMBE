#![allow(clippy::type_complexity)]
use mea::mpsc::TryRecvError;
use mea::mpsc::{BoundedReceiver, BoundedSender};
use mea::mutex::Mutex;
use mea::rwlock::RwLock;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;

use crate::player::PlayerId;
use crate::utils::ShardedLockedData;

const SYSLOG_FIFO_MAX_SIZE: usize = 10;

type SyslogData = (PlayerId, f64, SyslogEvent);
pub struct Fifo<T> {
    list: [Option<T>; SYSLOG_FIFO_MAX_SIZE],
    push_ind: usize,
    pop_ind: usize,
    len: usize,
}

impl<T: Copy> Default for Fifo<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Fifo<T> {
    pub fn new() -> Fifo<T> {
        Fifo {
            len: 0,
            push_ind: 0,
            pop_ind: 0,
            list: [const { None }; SYSLOG_FIFO_MAX_SIZE],
        }
    }

    pub fn push(&mut self, data: T) {
        if (self.len > 0) && (self.push_ind == self.pop_ind) {
            self.pop_ind = (self.pop_ind + 1) % SYSLOG_FIFO_MAX_SIZE;
        }
        *self.list.get_mut(self.push_ind).unwrap() = Some(data);
        self.push_ind = (self.push_ind + 1) % SYSLOG_FIFO_MAX_SIZE;
        self.len = (self.len + 1).min(SYSLOG_FIFO_MAX_SIZE);
    }

    fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }

        let data = std::mem::take(self.list.get_mut(self.pop_ind).unwrap());
        self.pop_ind = (self.pop_ind + 1) % SYSLOG_FIFO_MAX_SIZE;
        self.len -= 1;
        data
    }

    pub fn remove_all(&mut self) -> Vec<T> {
        let mut data = vec![];
        while self.len > 0 {
            let Some(got) = self.pop() else {
                continue;
            };
            data.push(got);
        }
        data
    }
}

#[derive(Clone)]
pub struct SyslogSend {
    sender: BoundedSender<SyslogData>,
    tstart: std::time::Instant,
}

impl SyslogSend {
    pub fn channel() -> (SyslogSend, SyslogRecv) {
        let (sender, recv) = mea::mpsc::bounded(1000);
        let tstart = std::time::Instant::now();
        let syslogsend = SyslogSend { sender, tstart };
        (syslogsend, SyslogRecv::init(recv, tstart))
    }

    pub async fn event(&self, player: &PlayerId, evt: SyslogEvent) {
        let ns = self.tstart.elapsed().as_secs_f64();
        self.sender.send((*player, ns, evt)).await.unwrap();
    }
}

pub type SyslogFifo =
    Arc<RwLock<ShardedLockedData<PlayerId, Arc<RwLock<Fifo<(f64, SyslogEvent)>>>>>>;

pub struct SyslogRecv {
    recv: Mutex<BoundedReceiver<SyslogData>>,
    pub(crate) fifo: SyslogFifo,
    tstart: std::time::Instant,
}

impl SyslogRecv {
    pub fn init(recv: BoundedReceiver<SyslogData>, tstart: std::time::Instant) -> SyslogRecv {
        SyslogRecv {
            recv: Mutex::new(recv),
            tstart,
            fifo: Arc::new(RwLock::new(ShardedLockedData::new(100))),
        }
    }

    pub async fn update(&self) {
        loop {
            match self.recv.lock().await.try_recv() {
                Ok((id, ns, evt)) => self.add_to_fifo(id, ns, evt).await,
                Err(TryRecvError::Empty) => break,
                Err(e) => {
                    let msg = format!("Error while receiving syslog: {e:?}");
                    log::error!("{}", msg);
                    panic!("{}", msg);
                }
            }
        }
    }

    pub async fn event(&self, player: PlayerId, evt: SyslogEvent) {
        self.add_to_fifo(player, self.tstart.elapsed().as_secs_f64(), evt)
            .await;
    }

    async fn add_to_fifo(&self, id: PlayerId, ns: f64, evt: SyslogEvent) {
        log::debug!("Player {id} got event {evt:?}");
        let sysfifo = self.fifo.read().await;
        let (reqadd, fifo) = if let Some(fifo) = sysfifo.clone_val(&id).await {
            (false, fifo)
        } else {
            (true, Arc::new(RwLock::new(Fifo::new())))
        };
        let mut player_fifo = fifo.write().await;
        player_fifo.push((ns, evt));
        drop(sysfifo);
        if reqadd {
            self.fifo.write().await.insert(id, fifo.clone()).await;
        }
    }
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, IntoStaticStr)]
pub enum SyslogEvent {
    #[default]
    Placeholder,

    // General game events
    GameStarted,
    GameLost,

    // Ship
    ShipDestroyed(crate::ship::ShipId),
    ShipFlightFinished(crate::ship::ShipId),
    ExtractionStopped(crate::ship::ShipId),

    // Warnings
    UnloadedNothing {
        station_cargo: crate::ship::cargo::ShipCargo,
        ship_cargo: crate::ship::cargo::ShipCargo,
    },
    LowFunds(std::time::Duration),
}

#[test]
fn test_syslog_fifo() {
    let mut fifo = Fifo::<usize>::new();

    fifo.push(0);
    assert_eq!(fifo.remove_all(), vec![0]);

    let ntest = SYSLOG_FIFO_MAX_SIZE + 5;
    for n in 0..ntest {
        fifo.push(n);
        println!("{} {} {:?}", fifo.len, fifo.push_ind, fifo.list);
        assert_eq!(fifo.len, (n + 1).min(SYSLOG_FIFO_MAX_SIZE), "iter {}", n);
    }

    println!();
    println!("{} {} {}", fifo.len, fifo.push_ind, fifo.pop_ind);

    for n in 0..ntest {
        let got = fifo.pop();
        println!("{} {} {:?}", fifo.len, fifo.pop_ind, fifo.list);
        assert_eq!(
            fifo.len,
            SYSLOG_FIFO_MAX_SIZE.saturating_sub(n + 1),
            "iter {}",
            n
        );

        if n < SYSLOG_FIFO_MAX_SIZE {
            assert_eq!(got, Some((ntest + n) - SYSLOG_FIFO_MAX_SIZE), "iter {}", n);
        } else {
            assert_eq!(got, None, "iter {}", n);
        }
    }

    for n in 0..(2 * ntest) {
        fifo.push(n);
        println!("{} {} {:?}", fifo.len, fifo.push_ind, fifo.list);
        assert_eq!(fifo.len, (n + 1).min(SYSLOG_FIFO_MAX_SIZE), "iter {}", n);
    }

    println!();
    fifo.push(usize::MAX);
    let all = fifo.remove_all();
    println!("{all:?}");
    assert_eq!(all.len(), SYSLOG_FIFO_MAX_SIZE);
    assert_eq!(
        all.first(),
        Some(&(((2 * ntest) + 1) - SYSLOG_FIFO_MAX_SIZE))
    );
    assert_eq!(all.last(), Some(&usize::MAX));
}
