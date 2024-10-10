use wasmtime_wasi::pipe::MemoryOutputPipe;

pub enum ComponentEvent {
    Execution {
        username_component_name: String,
        stdout: Option<MemoryOutputPipe>,
        duration: u128,
        status: u16
    }
}

pub fn default_event_handler(message: ComponentEvent) {
    match message {
        ComponentEvent::Execution { stdout, username_component_name, duration, status: _ } => {
            if let Some(stdout) = stdout {
                println!("Stdout from {username_component_name}: {}", String::from_utf8(stdout.contents().to_vec()).unwrap());
            }
            println!("Finished {username_component_name} in {duration}ms");
        }
    }
}