use tokio::sync::mpsc::Sender;
use super::{component_events::ComponentEvent, component_registry::ComponentRegistry};

pub struct ComponentImports {
    pub call_stack: Vec<String>,
    pub event_sender: Sender<ComponentEvent>,
    pub component_registry: ComponentRegistry
}

impl Clone for ComponentImports {
    fn clone(&self) -> Self {
        Self {
            call_stack: self.call_stack.clone(),
            event_sender: self.event_sender.clone(),
            component_registry: self.component_registry.clone()
        }
    }
}