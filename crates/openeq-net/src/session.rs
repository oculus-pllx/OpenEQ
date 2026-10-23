//! Shared connection configuration for the game and the headless zone probe.
use crate::{
    login::{LoginClient, Session},
    protocol::{ClientFamily, Protocol},
    world::WorldClient,
    zone::ZoneClient,
};
use anyhow::{Context, bail};
use serde::Deserialize;
use std::{net::SocketAddr, path::Path};

// Deliberately no Debug implementation: this contains a password.
#[derive(Clone, Deserialize)]
pub struct ConnectionConfig {
    pub host: String,
    #[serde(default = "login_port")]
    pub login_port: u16,
    #[serde(default = "world_port")]
    pub world_port: u16,
    pub username: String,
    pub password: String,
    pub character: String,
    pub server_id: Option<u32>,
    #[serde(default)]
    pub family: ClientFamily,
}

// Deliberately no Debug implementation: this contains a password.
#[derive(Clone, Deserialize)]
pub struct RosterConfig {
    pub host: String,
    #[serde(default = "login_port")]
    pub login_port: u16,
    #[serde(default = "world_port")]
    pub world_port: u16,
    pub username: String,
    pub password: String,
    pub server_id: Option<u32>,
    #[serde(default)]
    pub family: ClientFamily,
}
fn login_port() -> u16 {
    5999
}
fn world_port() -> u16 {
    9000
}

impl RosterConfig {
    pub fn from_reader(reader: impl std::io::Read) -> anyhow::Result<Self> {
        let config: Self =
            serde_json::from_reader(reader).context("invalid roster connection config")?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        for (name, value) in [
            ("host", self.host.as_str()),
            ("username", self.username.as_str()),
            ("password", self.password.as_str()),
        ] {
            if value.is_empty() {
                bail!("{name} must not be empty");
            }
        }
        let protocol = Protocol::for_family(self.family);
        if self.login_port != protocol.login_port {
            bail!("login_port does not match the selected client family");
        }
        if self.world_port != protocol.world_port {
            bail!("world_port does not match the selected client family");
        }
        Ok(())
    }
}

/// Enter a selected character while preserving authenticated later zoning.
/// Keep this future and its returned ZoneClient on the same live Tokio runtime
/// as the world connection: its UDP reader/ticker tasks belong to that runtime.
pub async fn enter_character(
    world: &mut WorldClient,
    world_address: SocketAddr,
    session: &Session,
    character: &str,
) -> anyhow::Result<ZoneClient> {
    let address = world.enter_world(character).await?;
    tracing::info!(%address, character, "entering zone");
    let mut zone = ZoneClient::connect(address, character).await?;
    zone.enable_zoning(world_address, session.account_id, session.key.clone());
    Ok(zone)
}

impl ConnectionConfig {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading connection config {}", path.display()))?;
        serde_json::from_slice(&bytes).context("invalid connection config")
    }
    pub async fn connect(&self) -> anyhow::Result<ZoneClient> {
        let addr = tokio::net::lookup_host((self.host.as_str(), self.login_port))
            .await?
            .find(|addr| addr.is_ipv4())
            .context("login hostname has no IPv4 address")?;
        tracing::info!(%addr, "connecting to login server");
        let mut login = LoginClient::connect(addr).await?;
        let session = login.login(&self.username, &self.password).await?;
        tracing::info!(account_id = session.account_id, "authenticated");
        let servers = login.server_list().await?;
        let server = servers
            .iter()
            .find(|s| self.server_id.map_or(s.is_up(), |id| s.server_id == id))
            .context("requested world is unavailable")?;
        login.play(server.server_id).await?;
        let world_address = SocketAddr::new(server.address, self.world_port);
        let mut world = WorldClient::connect_for_family(
            world_address,
            session.account_id,
            &session.key,
            self.family,
        )
        .await?;
        let characters = world.characters().await?;
        tracing::info!(names = ?characters.iter().map(|c| &c.name).collect::<Vec<_>>(), "character list");
        let selected = characters
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&self.character))
            .with_context(|| format!("character {} is not on this account", self.character))?;
        if !selected.enabled {
            bail!(
                "character {} is currently disabled by the server",
                selected.name
            );
        }
        enter_character(&mut world, world_address, &session, &self.character).await
    }
}

#[cfg(test)]
mod parallax_tests {
    use super::*;
    use crate::protocol::ClientFamily;

    macro_rules! assert_not_debug {
        ($type:ty) => {{
            struct Check;
            trait AmbiguousIfDebug<A> {
                fn check() {}
            }
            impl<T: ?Sized> AmbiguousIfDebug<()> for T {}
            impl<T: ?Sized + std::fmt::Debug> AmbiguousIfDebug<Check> for T {}
            let _ = <$type as AmbiguousIfDebug<_>>::check;
        }};
    }

    #[test]
    fn stdin_shape_defaults_to_rof2_and_never_requires_a_character() {
        let config = RosterConfig::from_reader(
            br#"{"host":"192.168.0.78","username":"user","password":"value"}"#.as_slice(),
        )
        .unwrap();

        assert_eq!(config.family, ClientFamily::Rof2);
        assert_eq!(config.login_port, 5999);
        assert_eq!(config.world_port, 9000);
        assert_eq!(config.server_id, None);
    }

    #[test]
    fn roster_config_rejects_empty_fields_wrong_ports_and_redacts_errors() {
        for document in [
            r#"{"host":"","username":"user","password":"sensitive-value"}"#,
            r#"{"host":"server","username":"","password":"sensitive-value"}"#,
            r#"{"host":"server","username":"user","password":""}"#,
            r#"{"host":"server","login_port":6000,"username":"user","password":"sensitive-value"}"#,
            r#"{"host":"server","world_port":9001,"username":"user","password":"sensitive-value"}"#,
        ] {
            let error = match RosterConfig::from_reader(document.as_bytes()) {
                Ok(_) => panic!("invalid roster config was accepted"),
                Err(error) => error,
            };
            assert!(!error.to_string().contains("sensitive-value"));
        }
    }

    #[test]
    fn credential_bearing_configs_do_not_implement_debug() {
        assert_not_debug!(ConnectionConfig);
        assert_not_debug!(RosterConfig);
    }
}
