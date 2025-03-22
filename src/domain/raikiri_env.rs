use vfs::FileSystem;

pub struct RaikiriEnvironment {
    pub fs: dyn FileSystem
}