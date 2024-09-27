use tokio::sync::mpsc::Sender;
use super::{component_events::ComponentEvent, component_registry::ComponentRegistry};

pub struct ComponentImports {
    pub call_stack: Vec<String>,
    pub event_sender: Sender<ComponentEvent>,
    pub component_registry: ComponentRegistry
}