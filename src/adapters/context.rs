use tokio::sync::mpsc::Sender;

use crate::ComponentEvent;

use super::component_registry::ComponentRegistry;

pub trait RaikiriContext {
    fn call_stack(&self) -> Vec<String>;
    fn event_sender(&self) -> Sender<ComponentEvent>;
    fn component_registry(&self) -> ComponentRegistry;
}