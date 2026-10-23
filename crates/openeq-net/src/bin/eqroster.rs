use anyhow::Context;
use openeq_net::{
    login::{LoginClient, ServerEntry},
    protocol::{ClientFamily, Protocol},
    session::RosterConfig,
    world::{Character, WorldClient},
};
use serde::Serialize;
use std::{net::SocketAddr, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMode {
    Stdin,
}

fn input_mode<I, S>(args: I) -> anyhow::Result<InputMode>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments: Vec<String> = args
        .into_iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    if arguments.len() == 2 && arguments[1] == "--config-stdin" {
        Ok(InputMode::Stdin)
    } else {
        anyhow::bail!("usage: eqroster --config-stdin")
    }
}

#[derive(Serialize)]
struct RosterResult {
    schema: u8,
    family: ClientFamily,
    login: String,
    world: WorldResult,
    roster: RosterSummary,
    entered_character: bool,
}

#[derive(Serialize)]
struct WorldResult {
    id: u32,
    name: String,
    status: &'static str,
}

#[derive(Serialize)]
struct RosterSummary {
    characters: usize,
    enabled: usize,
}

impl RosterResult {
    fn from_live(config: &RosterConfig, server: &ServerEntry, characters: &[Character]) -> Self {
        let protocol = Protocol::for_family(config.family);
        Self {
            schema: 1,
            family: protocol.family,
            login: format!("{}:{}", config.host, protocol.login_port),
            world: WorldResult {
                id: server.server_id,
                name: server.name.clone(),
                status: if server.is_up() { "up" } else { "down" },
            },
            roster: RosterSummary {
                characters: characters.len(),
                enabled: characters
                    .iter()
                    .filter(|character| character.enabled)
                    .count(),
            },
            entered_character: false,
        }
    }

    #[cfg(test)]
    fn fixture() -> Self {
        Self {
            schema: 1,
            family: ClientFamily::Rof2,
            login: "192.168.0.78:5999".to_owned(),
            world: WorldResult {
                id: 1,
                name: "NMS Reference".to_owned(),
                status: "up",
            },
            roster: RosterSummary {
                characters: 0,
                enabled: 0,
            },
            entered_character: false,
        }
    }
}

async fn probe(config: RosterConfig) -> anyhow::Result<RosterResult> {
    let address = tokio::net::lookup_host((config.host.as_str(), config.login_port))
        .await?
        .find(SocketAddr::is_ipv4)
        .context("IPv4 login endpoint")?;
    let mut login = LoginClient::connect(address).await?;
    let session = login.login(&config.username, &config.password).await?;
    let servers = login.server_list().await?;
    let server = servers
        .iter()
        .find(|server| {
            config
                .server_id
                .map_or(server.is_up(), |id| server.server_id == id)
        })
        .context("requested world is unavailable")?;
    login.play(server.server_id).await?;
    let mut world = WorldClient::connect_for_family(
        SocketAddr::new(server.address, config.world_port),
        session.account_id,
        &session.key,
        config.family,
    )
    .await?;
    let characters = world.characters().await?;
    let result = RosterResult::from_live(&config, server, &characters);
    world.disconnect().await?;
    login.disconnect().await?;
    Ok(result)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "openeq_net=info".into()),
        )
        .init();
    input_mode(std::env::args())?;
    let config = RosterConfig::from_reader(std::io::stdin().lock())?;
    let result = tokio::time::timeout(Duration::from_secs(60), probe(config))
        .await
        .context("roster probe timed out")??;
    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_only_the_stdin_configuration_mode() {
        assert_eq!(
            input_mode(["eqroster", "--config-stdin"]).unwrap(),
            InputMode::Stdin
        );
        assert!(input_mode(["eqroster", "--config", "/tmp/config"]).is_err());
        assert!(input_mode(["eqroster"]).is_err());
        assert!(input_mode(["eqroster", "--config-stdin", "extra"]).is_err());
    }

    #[test]
    fn roster_result_is_exactly_sanitized() {
        let json = serde_json::to_string(&RosterResult::fixture()).unwrap();

        assert_eq!(
            json,
            r#"{"schema":1,"family":"rof2","login":"192.168.0.78:5999","world":{"id":1,"name":"NMS Reference","status":"up"},"roster":{"characters":0,"enabled":0},"entered_character":false}"#
        );
        assert!(json.contains(r#""schema":1"#));
        assert!(json.contains(r#""family":"rof2""#));
        assert!(json.contains(r#""login":"192.168.0.78:5999""#));
        assert!(json.contains(r#""entered_character":false"#));
        assert!(!json.contains("username"));
        assert!(!json.contains("password"));
        assert!(!json.contains("session"));
        assert!(!json.contains("character_name"));
    }
}
