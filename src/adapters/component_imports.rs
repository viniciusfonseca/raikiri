use tokio::sync::mpsc::Sender;
use super::component_events::ComponentEvent;

pub struct ComponentImports {
    pub call_stack: Vec<String>,
    pub event_sender: Sender<ComponentEvent>,
}

// impl wit::Http:: for ComponentImports