use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientFamily {
    #[default]
    Rof2,
}

pub struct Protocol {
    pub family: ClientFamily,
    pub login_port: u16,
    pub world_port: u16,
    pub world_login_info_size: usize,
}

impl Protocol {
    pub const fn for_family(family: ClientFamily) -> Self {
        match family {
            ClientFamily::Rof2 => Self {
                family,
                login_port: 5999,
                world_port: 9000,
                world_login_info_size: 464,
            },
        }
    }
}

impl fmt::Display for ClientFamily {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rof2 => formatter.write_str("rof2"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unsupported client family")]
pub struct UnsupportedClientFamily;

impl FromStr for ClientFamily {
    type Err = UnsupportedClientFamily;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "rof2" => Ok(Self::Rof2),
            _ => Err(UnsupportedClientFamily),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rof2_is_the_only_supported_family_and_owns_wire_constants() {
        let protocol = Protocol::for_family(ClientFamily::Rof2);

        assert_eq!(protocol.family, ClientFamily::Rof2);
        assert_eq!(protocol.login_port, 5999);
        assert_eq!(protocol.world_port, 9000);
        assert_eq!(protocol.world_login_info_size, 464);
        assert_eq!(ClientFamily::Rof2.to_string(), "rof2");
        assert_eq!("rof2".parse::<ClientFamily>().unwrap(), ClientFamily::Rof2);
        assert!("tob".parse::<ClientFamily>().is_err());
    }
}
