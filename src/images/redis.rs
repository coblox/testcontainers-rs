use crate::{core::WaitFor, Image};

const NAME: &str = "redis";
const DEFAULT_TAG: &str = "5.0";

#[derive(Debug, Default, Clone)]
pub struct RedisArgs;

impl IntoIterator for RedisArgs {
    type Item = String;
    type IntoIter = ::std::vec::IntoIter<String>;

    fn into_iter(self) -> <Self as IntoIterator>::IntoIter {
        vec![].into_iter()
    }
}

#[derive(Debug)]
pub struct Redis {
    tag: String,
}

impl Default for Redis {
    fn default() -> Self {
        Redis {
            tag: DEFAULT_TAG.to_string(),
        }
    }
}

impl Image for Redis {
    type Args = RedisArgs;

    fn name(&self) -> String {
        NAME.to_owned()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stdout("Ready to accept connections")]
    }
}
