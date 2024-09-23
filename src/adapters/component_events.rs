use wasmtime_wasi::pipe::MemoryOutputPipe;

pub enum ComponentEvent {
    Stdout {
        username_component_name: String,
        stdout: MemoryOutputPipe
    }
}