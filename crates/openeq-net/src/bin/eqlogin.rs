//! Headless end-to-end login / NPC movement probe. Credentials stay in a file.
use openeq_net::{session::ConnectionConfig, zone::ZoneEvent};
use std::{collections::HashMap, path::PathBuf, time::Duration};

#[derive(Debug, Eq, PartialEq)]
enum ConfigInput {
    File(PathBuf),
    Stdin,
}

struct Options {
    input: ConfigInput,
    seconds: u64,
}

fn parse_options<I, S>(args: I) -> anyhow::Result<Options>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let values: Vec<String> = args
        .into_iter()
        .map(|value| value.as_ref().to_owned())
        .collect();
    let mut args = values.into_iter();
    let mut input = None;
    let mut seconds = 30u64;
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--config" if input.is_none() => {
                input = Some(ConfigInput::File(PathBuf::from(
                    args.next()
                        .ok_or_else(|| anyhow::anyhow!("missing config path"))?,
                )));
            }
            "--config-stdin" if input.is_none() => input = Some(ConfigInput::Stdin),
            "--seconds" => {
                seconds = args
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("missing duration"))?
                    .parse()?;
            }
            _ => anyhow::bail!("usage: eqlogin (--config FILE | --config-stdin) [--seconds 30]"),
        }
    }
    anyhow::ensure!(
        (5..=300).contains(&seconds),
        "duration must be 5 through 300 seconds"
    );
    Ok(Options {
        input: input.ok_or_else(|| anyhow::anyhow!("configuration source is required"))?,
        seconds,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "openeq_net=info".into()),
        )
        .init();
    let options = parse_options(std::env::args().skip(1))?;
    let config = match options.input {
        ConfigInput::File(path) => ConnectionConfig::load(&path)?,
        ConfigInput::Stdin => ConnectionConfig::from_reader(std::io::stdin().lock())?,
    };
    let mut zone = config.connect().await?;
    let mut spawns = HashMap::new();
    let mut moves = 0u64;
    let mut ready = false;
    let mut own = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(options.seconds);
    let mut heartbeat = tokio::time::interval(Duration::from_secs(2));
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => break,
            _ = heartbeat.tick() => if let Some((id, position)) = own { zone.send_position(id, position).await?; },
            event = zone.next_event() => match event? {
                ZoneEvent::Spawn(spawn) => {
                    if spawn.name.eq_ignore_ascii_case(&config.character) { own = Some((spawn.id, spawn.position)); }
                    if spawns.len() < 8 { println!("spawn {}: {} race={} at {:.1},{:.1},{:.1}", spawn.id, spawn.name, spawn.race, spawn.position.x, spawn.position.y, spawn.position.z); }
                    spawns.insert(spawn.id, spawn);
                }
                ZoneEvent::Movement { id, position } => {
                    if let Some(spawn) = spawns.get_mut(&id) {
                        if spawn.npc && (spawn.position.x != position.x || spawn.position.y != position.y || spawn.position.z != position.z) { moves += 1; }
                        spawn.position = position;
                    }
                }
                ZoneEvent::Despawn(id) => { spawns.remove(&id); }
                ZoneEvent::Environment(env) => println!("zone: {} ({}) fog={:?} range={:?}..{:?} sky={}", env.short_name, env.zone_id, env.fog_color[0], env.fog_start, env.fog_end, env.sky),
                ZoneEvent::Ready => { ready = true; println!("zone ready"); }
                ZoneEvent::Other { opcode, size } => tracing::debug!(opcode = format!("{opcode:#06x}"), size, "zone packet"),
                _ => {}
            }
        }
    }
    zone.logout().await?;
    let npcs = spawns.values().filter(|s| s.npc).count();
    println!(
        "RESULT ready={ready} entities={} npcs={npcs} npc_position_changes={moves}",
        spawns.len()
    );
    anyhow::ensure!(ready && npcs > 0, "zone entry / NPC streaming incomplete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_accept_one_config_source_and_bounded_duration() {
        let stdin = parse_options(["--config-stdin"]).unwrap();
        assert_eq!(stdin.input, ConfigInput::Stdin);
        assert_eq!(stdin.seconds, 30);

        let file = parse_options(["--config", "/tmp/config", "--seconds", "5"]).unwrap();
        assert_eq!(file.input, ConfigInput::File(PathBuf::from("/tmp/config")));
        assert_eq!(file.seconds, 5);

        for args in [
            vec!["--config-stdin", "--config", "/tmp/config"],
            vec!["--config-stdin", "--seconds", "4"],
            vec!["--config-stdin", "--seconds", "301"],
            vec![],
        ] {
            assert!(parse_options(args).is_err());
        }
    }
}
