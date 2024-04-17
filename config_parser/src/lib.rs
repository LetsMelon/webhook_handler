pub mod internal;
pub mod raw;

#[test]
fn can_parse_the_example_file() {
    let demo_config_file = include_bytes!("../../webhook_handler_demo_config.yml");

    crate::raw::ConfigFile::parse_from_reader(&demo_config_file[..]).unwrap();
}
