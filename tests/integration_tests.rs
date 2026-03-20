#[test]
fn demo_configuration_does_not_require_real_secrets() {
    let config = std::fs::read_to_string("clawlegion.toml").expect("config should exist");
    assert!(config.contains("mode = \"demo\""));
    assert!(config.contains("api_key_env = \"CLAWLEGION_LLM_API_KEY\""));
}

#[test]
fn org_defaults_are_neutral_demo_values() {
    let config = std::fs::read_to_string("config/org.toml").expect("org config should exist");
    assert!(config.contains("ClawLegion Demo Org"));
    assert!(config.contains("CEO"));
    assert!(config.contains("Researcher"));
    assert!(config.contains("Executor"));
}

