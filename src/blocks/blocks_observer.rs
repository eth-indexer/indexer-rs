use alloy::rpc::types::Block;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Arc};

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Event {
    Reorg,
    BlocksChanged,
}

pub type Subscriber = fn(blocks: Option<Vec<Block>>);

#[derive(Default)]
pub struct Publisher {
    events: HashMap<Event, Vec<Subscriber>>,
}

impl Publisher {
    pub fn subscribe(&mut self, event_type: Event, listener: Subscriber) {
        self.events.entry(event_type.clone()).or_default();
        self.events.get_mut(&event_type).unwrap().push(listener);
    }

    pub fn unsubscribe(&mut self, event_type: Event, listener: Subscriber) {
        self.events
            .get_mut(&event_type)
            .unwrap()
            .retain(|&x| x != listener);
    }

    pub fn reorg(&self) {
        let listeners = self.events.get(&Event::Reorg);
        let listeners = match listeners {
            Some(listeners) => listeners,
            None => return,
        };
        for listener in listeners {
            listener(None);
        }
    }

    pub fn blocks_changed(&self, blocks: Vec<Block>) {
        let listeners = self.events.get(&Event::BlocksChanged);
        let listeners = match listeners {
            Some(listeners) => listeners,
            None => return,
        };
        for listener in listeners {
            listener(Some(blocks.clone()));
        }
    }
}
