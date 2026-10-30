//! Regression tests for the optional single-player passenger connections setting.
use super::Settings;

#[test]
fn default_is_off_for_legacy_desktop_and_mobile() {
    assert!(!Settings::phone().passenger_connections);
    assert!(!Settings::desktop().passenger_connections);
    for src in ["", "version=1\n", "passenger_connections=0\n"] {
        let s = Settings::from_text(src);
        assert!(!s.passenger_connections);
        assert_eq!(s.passenger_transfer_percent, 20.0);
    }
}

#[test]
fn chance_clamps_and_never_implicitly_enables() {
    for (input, expected) in [("0", 0.0), ("20", 20.0), ("100", 100.0),
        ("999", 100.0), ("-10", 0.0), ("nan", 20.0)] {
        let s = Settings::from_text(&format!("passenger_transfer_percent={input}\n"));
        assert_eq!(s.passenger_transfer_percent, expected);
        assert!(!s.passenger_connections);
    }
}

#[test]
fn both_settings_writers_preserve_optional_preference_and_chance() {
    let key = "passenger_connections";
    for phone in [false, true] {
        assert_eq!(omsi_launcher_lib::settings_from_text_on(None, phone)[key], false);
        for on in [false, true, false] {
            let mut settings = Settings::default();
            settings.passenger_connections = on;
            settings.passenger_transfer_percent = 100.0;
            let native = settings.to_text();
            let mut launcher = omsi_launcher_lib::settings_from_text_on(Some(&native), phone);
            assert_eq!(launcher[key], on);
            assert_eq!(launcher["passenger_transfer_percent"], 100.0);
            launcher["passenger_transfer_percent"] = serde_json::json!(20.0);
            let text = omsi_launcher_lib::settings_to_text(&launcher, Some(&native));
            let restored = Settings::from_text(&text);
            assert_eq!(restored.passenger_connections, on);
            assert_eq!(restored.passenger_transfer_percent, 20.0);
            assert_eq!(text.lines().filter(|line| line.starts_with(&format!("{key}="))).count(), 1);
        }
    }
}
