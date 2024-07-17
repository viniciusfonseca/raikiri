use wasmtime_wasi::pipe::MemoryOutputPipe;

pub enum ModuleEvent {
    Stdout {
        username_module_name: String,
        stdout: MemoryOutputPipe
    }
}