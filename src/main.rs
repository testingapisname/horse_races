use std::collections::HashMap;
use std::sync::Arc;
use rand::rngs::OsRng;
use rand::seq::SliceRandom;
use russh::keys::{Certificate, *};
use russh::server::{Msg, Server as _, Session};
use russh::*;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .init();

    let config = russh::server::Config {
        inactivity_timeout: Some(Duration::from_secs(3600)),
        auth_rejection_time: Duration::from_secs(3),
        auth_rejection_time_initial: Some(Duration::from_secs(0)),
        keys: vec![russh::keys::PrivateKey::random(&mut OsRng, russh::keys::Algorithm::Ed25519)
            .expect("failed to generate host key")],
        preferred: Preferred::default(),
        ..Default::default()
    };
    let config = Arc::new(config);

    let mut sh = Server {
        clients: Arc::new(Mutex::new(HashMap::new())),
        player_horses: Arc::new(Mutex::new(HashMap::new())),
        usernames: Arc::new(Mutex::new(HashMap::new())),
        id: 0,
    };

    let socket = TcpListener::bind(("0.0.0.0", 2222))
        .await
        .expect("failed to bind to port 2222");

    let clients_for_race = sh.clients.clone();
    let player_horses_for_race = sh.player_horses.clone();
    let usernames_for_race = sh.usernames.clone();

    let server = sh.run_on_socket(config, &socket);
    let handle = server.handle();

    // 🏇 Start recurring races
    tokio::spawn(async move {
        sleep(Duration::from_secs(20)).await; // wait for players to join
        loop {
            run_race(
                clients_for_race.clone(),
                player_horses_for_race.clone(),
                usernames_for_race.clone(),
            )
            .await;
            sleep(Duration::from_secs(20)).await;
        }
    });

    // Auto-shutdown timer (10 min)
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(600)).await;
        handle.shutdown("Server shutting down after 10 minutes".into());
    });

    server.await.unwrap();
}

#[derive(Clone)]
struct Server {
    clients: Arc<Mutex<HashMap<usize, (ChannelId, russh::server::Handle)>>>,
    player_horses: Arc<Mutex<HashMap<usize, String>>>,
    usernames: Arc<Mutex<HashMap<usize, String>>>,
    id: usize,
}

impl Server {
    async fn broadcast(&self, msg: &str) {
        let data = CryptoVec::from(msg.as_bytes());
        let mut clients = self.clients.lock().await;
        for (_, (chan, s)) in clients.iter_mut() {
            let _ = s.data(*chan, data.clone()).await;
        }
    }
}

impl server::Server for Server {
    type Handler = Self;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> Self {
        let s = self.clone();
        self.id += 1;
        s
    }

    fn handle_session_error(&mut self, error: <Self::Handler as russh::server::Handler>::Error) {
        eprintln!("Session error: {:#?}", error);
    }
}

impl server::Handler for Server {
    type Error = russh::Error;

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let id = self.id;
        let handle = session.handle();
        {
            let mut clients = self.clients.lock().await;
            clients.insert(id, (channel.id(), handle.clone()));
        }

        // Welcome new player
        let chan_id = channel.id();
        tokio::spawn(async move {
            sleep(Duration::from_millis(300)).await;
            let welcome = concat!(
                "\r\n🐎  Welcome to Last Call Derby! 🐎\r\n",
                "=====================================\r\n",
                "You’ll be prompted to pick your horse before each race.\r\n",
                "Just type the horse number (1–4) when asked.\r\n\r\n"
            );
            let _ = handle.data(chan_id, CryptoVec::from(welcome.as_bytes())).await;
        });

        Ok(true)
    }

    async fn auth_publickey(
        &mut self,
        user: &str,
        _key: &ssh_key::PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        let mut names = self.usernames.lock().await;
        names.insert(self.id, user.to_string());
        Ok(server::Auth::Accept)
    }

    async fn auth_openssh_certificate(
        &mut self,
        user: &str,
        _certificate: &Certificate,
    ) -> Result<server::Auth, Self::Error> {
        let mut names = self.usernames.lock().await;
        names.insert(self.id, user.to_string());
        Ok(server::Auth::Accept)
    }

    async fn data(
        &mut self,
        channel: ChannelId,
        data: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if data == [3] {
            return Err(russh::Error::Disconnect);
        }

        let input = String::from_utf8_lossy(data).trim().to_string();
        let horses = vec!["Thunderhoof", "BeerGuzzler", "Shotglass", "Hangover Express"];

        // Handle horse selection only
        if let Ok(choice) = input.parse::<usize>() {
            if (1..=horses.len()).contains(&choice) {
                let pick = horses[choice - 1].to_string();
                let mut map = self.player_horses.lock().await;
                map.insert(self.id, pick.clone());
                drop(map);

                let names = self.usernames.lock().await;
                let name = names
                    .get(&self.id)
                    .cloned()
                    .unwrap_or_else(|| format!("Player #{}", self.id));
                drop(names);

                let msg = format!("\r\n✅ {} chose {}! Good luck!\r\n", name, pick);
                session.data(channel, CryptoVec::from(msg.as_bytes())).ok();

                // Broadcast to all players
                let broadcast = format!("📣 {} has picked {}!\r\n", name, pick);
                let mut clients = self.clients.lock().await;
                for (_, (chan, s)) in clients.iter_mut() {
                    let _ = s.data(*chan, CryptoVec::from(broadcast.as_bytes())).await;
                }

                return Ok(());
            }
        }

        Ok(())
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let id = self.id;
        let clients = self.clients.clone();
        tokio::spawn(async move {
            let mut clients = clients.lock().await;
            clients.remove(&id);
        });
    }
}

// 🏇 Race animation + finish line
async fn run_race(
    clients: Arc<Mutex<HashMap<usize, (ChannelId, russh::server::Handle)>>>,
    player_horses: Arc<Mutex<HashMap<usize, String>>>,
    usernames: Arc<Mutex<HashMap<usize, String>>>,
) {
    let horses = vec!["Thunderhoof", "BeerGuzzler", "Shotglass", "Hangover Express"];
    let track_length = 50;

    // Betting phase
    {
        let mut clients_lock = clients.lock().await;
        for (_id, (chan, s)) in clients_lock.iter_mut() {
            let msg = concat!(
                "\r\n🏁 A new race is about to start! Place your bets! 🏁\r\n",
                "Pick your horse by typing its number:\r\n",
                "  1. Thunderhoof\r\n",
                "  2. BeerGuzzler\r\n",
                "  3. Shotglass\r\n",
                "  4. Hangover Express\r\n",
                "You have 10 seconds before the race starts...\r\n\r\n"
            );
            let _ = s.data(*chan, CryptoVec::from(msg.as_bytes())).await;
        }
    }

    sleep(Duration::from_secs(10)).await;

    // Pick random winner
    let winner_index = rand::random::<usize>() % horses.len();
    let winner = horses[winner_index];

    let mut progress = vec![0; horses.len()];

    // Animate race
    loop {
        let mut frame = String::new();
        frame.push_str("\x1b[2J\x1b[H"); // clear screen
        frame.push_str("🏇 The Last Call Derby 🏇\r\n");
        frame.push_str("=".repeat(track_length + 10).as_str());
        frame.push_str("\r\n");

        let mut finished = true;
        for (i, horse) in horses.iter().enumerate() {
            // Speed bias: winner runs faster
            if progress[i] < track_length {
                finished = false;
                let speed = if *horse == winner {
                    2 + (rand::random::<u8>() % 2) // faster
                } else {
                    1 + (rand::random::<u8>() % 2)
                };
                progress[i] = (progress[i] + speed as usize).min(track_length);
            }

            let bar = "-".repeat(progress[i]) + "🐎";
            let spaces = " ".repeat(track_length.saturating_sub(progress[i]));
            frame.push_str(&format!("{:15}: {}{}|🏁|\r\n", horse, bar, spaces));
        }

        frame.push_str("=".repeat(track_length + 10).as_str());
        frame.push_str("\r\n(Press Ctrl+C to exit)\r\n");

        let data = CryptoVec::from(frame.as_bytes());
        let mut clients_lock = clients.lock().await;
        for (_, (chan, s)) in clients_lock.iter_mut() {
            let _ = s.data(*chan, data.clone()).await;
        }
        drop(clients_lock);

        if finished {
            break;
        }

        sleep(Duration::from_millis(200)).await;
    }

    // Announce results
    {
        let mut clients_lock = clients.lock().await;
        let map = player_horses.lock().await;
        let names = usernames.lock().await;

        for (id, (chan, s)) in clients_lock.iter_mut() {
            let name = names
                .get(id)
                .cloned()
                .unwrap_or_else(|| format!("Player #{}", id));

            match map.get(id) {
                Some(pick) if pick == winner => {
                    let msg = format!(
                        "\r\n🏆 Winner: {} 🏆\r\n🎉 Congrats {}, you picked the winner!\r\n",
                        winner, name
                    );
                    let _ = s.data(*chan, CryptoVec::from(msg.as_bytes())).await;
                }
                Some(pick) => {
                    let msg = format!(
                        "\r\n🏆 Winner: {} 🏆\r\n{} picked {} — better luck next time!\r\n",
                        winner, name, pick
                    );
                    let _ = s.data(*chan, CryptoVec::from(msg.as_bytes())).await;
                }
                None => {
                    let msg = format!(
                        "\r\n🏆 Winner: {} 🏆\r\n{} didn’t pick a horse — automatic drink!\r\n",
                        winner, name
                    );
                    let _ = s.data(*chan, CryptoVec::from(msg.as_bytes())).await;
                }
            }
        }
    }

    // Reset picks for next race
    {
        let mut map = player_horses.lock().await;
        map.clear();
    }
}
