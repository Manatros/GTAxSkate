//! The co-op link at run time: the UDP socket to the other player's
//! plugin, and what this game shows of them. Over FiveM their character is
//! the network ped FiveM keeps near where they skate, posed here with their
//! Skate pose; with no such ped (two story games on a LAN) a skater ped
//! stands in for them. Their board is drawn on it, its sounds play from
//! where they ride, and the objects they dropped are put down here too.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use bevy_math::Vec3;

use super::natives as n;
use crate::coop::{Packet, newer};

/// Seconds without a packet before the other player counts as gone.
const SILENCE: Duration = Duration::from_secs(2);
/// The ped standing in for the other player when the game has none.
pub const STAND_IN: &str = "a_m_y_skater_01";

pub struct Link {
    socket: UdpSocket,
    peer: SocketAddr,
    seq: u32,
    sent_at: Instant,
    /// The newest packet heard, and when.
    pub heard: Option<(Packet, Instant)>,
}

impl Link {
    /// Listens on `port` for `peer`; none without a usable socket.
    pub fn open(peer: &str, port: u16) -> Result<Self, String> {
        let peer: SocketAddr = peer.parse().map_err(|_| format!("CoopPeer {peer:?} is not an address:port"))?;
        let socket = UdpSocket::bind(("0.0.0.0", port)).map_err(|e| format!("co-op port {port}: {e}"))?;
        socket.set_nonblocking(true).map_err(|e| e.to_string())?;
        Ok(Self { socket, peer, seq: 0, sent_at: Instant::now() - Duration::from_secs(1), heard: None })
    }

    /// Sends this player's skater, at most `rate` times a second.
    pub fn send(&mut self, mut packet: Packet, rate: f32) {
        if self.sent_at.elapsed().as_secs_f32() < 1. / rate.max(0.5) {
            return;
        }
        self.sent_at = Instant::now();
        self.seq = self.seq.wrapping_add(1);
        packet.seq = self.seq;
        let _ = self.socket.send_to(&packet.encode(), self.peer);
    }

    /// Reads what arrived; keeps the newest packet from the peer's address.
    pub fn receive(&mut self) {
        let mut buffer = [0u8; 2048];
        while let Ok((length, from)) = self.socket.recv_from(&mut buffer) {
            if from.ip() != self.peer.ip() {
                continue;
            }
            let Some(packet) = Packet::decode(&buffer[..length]) else { continue };
            let fresh = self.heard.as_ref().is_none_or(|(old, at)| newer(packet.seq, old.seq) || at.elapsed() > SILENCE);
            if fresh {
                self.heard = Some((packet, Instant::now()));
            }
        }
    }

    /// The other player's latest skater, while they are heard.
    pub fn current(&self) -> Option<&Packet> {
        self.heard.as_ref().filter(|(_, at)| at.elapsed() < SILENCE).map(|(p, _)| p)
    }
}

/// The ped of another player near `at` in a networked game (FiveM), if any.
pub fn network_ped_near(at: Vec3, within: f32) -> Option<i32> {
    let me = n::player_id();
    (0..32)
        .filter(|&player| player != me && n::player_active(player))
        .map(n::player_ped_of)
        .filter(|&ped| ped != 0 && n::entity_exists(ped))
        .map(|ped| (ped, n::entity_coords(ped).distance(at)))
        .filter(|(_, d)| *d <= within)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(ped, _)| ped)
}
