use wasmtime_wasi::pipe::MemoryOutputPipe;
use chrono::DateTime;

pub enum ComponentEvent {
    Execution {
        username_component_name: String,
        stdout: Option<MemoryOutputPipe>,
        start: DateTime<chrono::Utc>,
        duration: u128,
        status: u16
    }
}

pub fn default_event_handler(message: ComponentEvent) {
    match message {
        ComponentEvent::Execution { stdout, username_component_name, start, duration, status } => {
            if let Some(stdout) = stdout {
                println!("Stdout from {username_component_name}: {}", String::from_utf8(stdout.contents().to_vec()).unwrap());
            }
            let start_text = start.to_rfc3339();
            println!("Started {username_component_name} at {start_text} and finished in {duration}ms. Status code: {status}");
        }
    }
}
