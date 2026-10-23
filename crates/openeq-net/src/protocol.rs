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
