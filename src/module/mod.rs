use proc_qq::Module;

mod ping;
mod video;

pub fn get_module() -> Vec<Module> {
    vec![ping::module(), video::module()]
}
