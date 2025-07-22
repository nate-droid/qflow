use crate::Gate;

#[derive(Debug, Clone)]
pub struct Circuit {
    pub gates: Vec<Gate>,
}

impl Circuit {
    pub fn new() -> Self {
        Self { gates: Vec::new() }
    }

    pub fn add_gate(&mut self, gate: Gate) {
        self.gates.push(gate);
    }
}