pub mod action;
pub mod command;
pub mod events;

pub struct Context {
    pub channel_name: &'static str,
    pub helix_auth: String,
}
