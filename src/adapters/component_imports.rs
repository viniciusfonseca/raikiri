use tokio::sync::mpsc::Sender;
use super::{component_events::ComponentEvent, component_registry::ComponentRegistry, context::RaikiriContext};

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

impl RaikiriContext for ComponentImports {
    fn call_stack(&self) -> &Vec<String> {
        &self.call_stack
    }

    fn event_sender(&self) -> &Sender<ComponentEvent> {
        &self.event_sender
    }

    fn component_registry(&self) -> &ComponentRegistry {
        &self.component_registry
    }
}