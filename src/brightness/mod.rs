use source::{BrightnessSource, default_source};
use value::notify;

pub use source::{KEYBOARD_ID, MONITOR_ID};
pub use value::{
    Brightness, BrightnessGetCommands, BrightnessItem, BrightnessSetCommands, evaluate_item, match_get_commands,
    match_set_commands,
};

mod source;
mod value;
