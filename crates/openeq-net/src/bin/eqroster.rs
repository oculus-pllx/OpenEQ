fn main() {}

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
