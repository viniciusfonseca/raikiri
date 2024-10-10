use wasmtime_wasi::pipe::MemoryOutputPipe;

pub enum ComponentEvent {
    Execution {
        username_component_name: String,
        stdout: Option<MemoryOutputPipe>,
        duration: u128,
        status: u16
    }
}