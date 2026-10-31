//! A candidate room owns its transport until the running game accepts the handover.

use omsi_net::{LanSession, Pose, WorldInfo};
use std::time::{Duration, Instant};

pub struct PreparedJoin {
    pub session: LanSession,
    target: String,
    client: Option<(omsi_net::ws::WsClient, String)>,
}

impl PreparedJoin {
    pub fn target(&self) -> &str {
        &self.target
    }

    /// Establish a new room without changing the active room's gateway or offers.
    pub fn connect(
        target: &str,
        name: &str,
        world: WorldInfo,
        planned: &Pose,
    ) -> Result<Self, String> {
        Self::connect_with_wait(target, name, world, planned, Duration::from_secs(10))
    }

    /// Keep the hosted room alive when preparing a handover to another room.
    pub fn connect_for_handover(
        target: &str,
        name: &str,
        world: WorldInfo,
        planned: &Pose,
        hosted_session: Option<u64>,
    ) -> Result<Self, String> {
        const SELF_JOIN: &str = "You cannot join the room you are hosting";
        if omsi_net::SessionCode::decode(target.trim())
            .ok()
            .is_some_and(|code| hosted_session == Some(code.session))
        {
            return Err(SELF_JOIN.into());
        }
        let candidate = Self::connect(target, name, world, planned)?;
        // Addresses and gateways need not include a code; check the negotiated identity too.
        if hosted_session == Some(candidate.session.session) {
            return Err(SELF_JOIN.into());
        }
        Ok(candidate)
    }

    fn connect_with_wait(
        target: &str,
        name: &str,
        world: WorldInfo,
        planned: &Pose,
        wait: Duration,
    ) -> Result<Self, String> {
        let target = target.trim();
        if target.is_empty() || target.len() > 2048 || target.chars().any(char::is_control) {
            return Err("Enter a valid connection code or server address".into());
        }
        let target = omsi_net::official::resolve_target(target)?;
        crate::bans::announce_local_hardware_id();
        let mut candidate = Self::open(&target, name, world.clone())?;
        match candidate.wait(planned, wait) {
            Ok(()) => Ok(candidate),
            Err(error)
                if candidate.session.turned_away.is_none()
                    && candidate.session.welcome.is_none() =>
            {
                let tunnel = omsi_net::SessionCode::decode(&target)
                    .ok()
                    .and_then(|code| omsi_net::bridge::lookup_tunnel(code.session));
                let Some(tunnel) = tunnel else {
                    return Err(error);
                };
                let mut fallback = Self::open(&tunnel, name, world)?;
                fallback.target = target;
                fallback.wait(planned, wait)?;
                Ok(fallback)
            }
            Err(error) => Err(error),
        }
    }

    fn open(target: &str, name: &str, world: WorldInfo) -> Result<Self, String> {
        let (direct, client) = match omsi_net::ws::ws_url(target) {
            Some(url) => {
                let client = omsi_net::ws::WsClient::connect(&url)?;
                (client.local.to_string(), Some((client, url)))
            }
            None => (target.to_string(), None),
        };
        let session = LanSession::join(&direct, name, world, Duration::from_secs(3))?;
        Ok(Self {
            session,
            target: target.to_string(),
            client,
        })
    }

    fn wait(&mut self, planned: &Pose, wait: Duration) -> Result<(), String> {
        let started = Instant::now();
        while started.elapsed() < wait {
            self.session.tick(0.02, planned);
            if let Some(why) = self
                .session
                .turned_away
                .as_ref()
                .or(self.session.rejected.as_ref())
            {
                return Err(why.clone());
            }
            if self.session.connected && self.session.welcome.is_some() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        Err("The room did not answer; the current game has not been changed".into())
    }

    /// Commit only after the game has accepted the room and any required map change.
    pub fn accept(self) -> LanSession {
        crate::gateway::replace_join_transport(self.client);
        let offers = crate::gateway::new_offers_session();
        crate::gateway::ask_server_offers(&self.target, offers);
        self.session
    }
}

#[cfg(test)]
mod tests;
