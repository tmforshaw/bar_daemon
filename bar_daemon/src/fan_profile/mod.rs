use source::{FAN_STATE_STRINGS, FanProfileSource, default_source};
use value::FanState;

pub use value::{
    FanProfile, FanProfileGetCommands, FanProfileItem, FanProfileSetCommands, evaluate_item, match_get_commands,
    match_set_commands,
};

mod source;
mod value;
