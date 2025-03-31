use crate::raydium::Raydium;

pub struct ModuleBase {
    pub scope: Raydium,
}
impl ModuleBase {
    pub fn new(scope: Raydium) -> Self {
        Self { scope }
    }
}
