use qsim::Gate;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pauli {
    I,
    X,
    Y,
    Z,
}

fn pauli_to_gate(pauli: Pauli) -> Gate {
    match pauli {
        Pauli::I => Gate::I { qubit: 0 },
        Pauli::X => Gate::X { qubit: 0 },
        Pauli::Y => Gate::Y { qubit: 0 },
        Pauli::Z => Gate::Z { qubit: 0 },
    }
}

impl fmt::Display for Pauli {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PauliTerm {
    pub coefficient: f64,
    pub operators: Vec<(Pauli, usize)>, // Vec of (Pauli type, qubit index)
}

impl PauliTerm {
    pub fn new() -> Self {
        PauliTerm {
            coefficient: 1.0,
            operators: Vec::new(),
        }
    }

    pub fn with_pauli(mut self, qubit_index: usize, pauli: Pauli) -> Self {
        if pauli != Pauli::I {
            self.operators.push((pauli, qubit_index));
            self.operators.sort_by_key(|&(_, q_idx)| q_idx);
        }
        self
    }

    pub fn with_coefficient(mut self, coefficient: f64) -> Self {
        self.coefficient = coefficient;
        self
    }
}

impl Default for PauliTerm {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct PauliTermParseError;

impl FromStr for PauliTerm {
    type Err = PauliTermParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('*').map(|p| p.trim()).collect();
        if parts.len() != 2 {
            return Err(PauliTermParseError);
        }

        let coefficient = parts[0].parse::<f64>().map_err(|_| PauliTermParseError)?;
        let operator_str = parts[1];

        let mut term = PauliTerm::new().with_coefficient(coefficient);

        for op in operator_str.split_whitespace() {
            if op.is_empty() || op.len() < 2 {
                return Err(PauliTermParseError);
            }
            let (pauli_char, qubit_idx_str) = op.split_at(1);
            let qubit_index = qubit_idx_str
                .parse::<usize>()
                .map_err(|_| PauliTermParseError)?;

            let pauli = match pauli_char {
                "X" | "x" => Pauli::X,
                "Y" | "y" => Pauli::Y,
                "Z" | "z" => Pauli::Z,
                "I" | "i" => Pauli::I,
                _ => return Err(PauliTermParseError),
            };
            term = term.with_pauli(qubit_index, pauli);
        }

        Ok(term)
    }
}

impl fmt::Display for PauliTerm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.8}", self.coefficient)?;
        if !self.operators.is_empty() {
            write!(f, " *")?;
            for (pauli, qubit_index) in &self.operators {
                write!(f, " {}{}", pauli, qubit_index)?;
            }
        }
        Ok(())
    }
}

// Hamiltonian represents a sum of Pauli terms, which can be used to describe quantum systems.
#[derive(Debug, Clone, Default)]
pub struct Hamiltonian {
    pub terms: Vec<PauliTerm>,
}

impl Hamiltonian {
    pub fn new() -> Self {
        Hamiltonian { terms: Vec::new() }
    }

    pub fn add_term(&mut self, term: PauliTerm) {
        self.terms.push(term);
    }

    pub fn with_term(mut self, term: PauliTerm) -> Self {
        self.add_term(term);
        self
    }
}

/// Display trait for the entire Hamiltonian.
impl fmt::Display for Hamiltonian {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, term) in self.terms.iter().enumerate() {
            if i > 0 {
                write!(f, "\n+ ")?;
            }
            write!(f, "{}", term)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pauli_term() {
        let term_str = "0.5 * X0 Z1";
        let term = PauliTerm::from_str(term_str).unwrap();
        assert_eq!(term.coefficient, 0.5);
        assert_eq!(term.operators, vec![(Pauli::X, 0), (Pauli::Z, 1)]);
    }

    #[test]
    fn test_hamiltonian_display() {
        let h2_hamiltonian = Hamiltonian::new()
            .with_term(PauliTerm::from_str("-0.8126 * I0").unwrap())
            .with_term(PauliTerm::from_str("0.1712 * Z0").unwrap())
            .with_term(PauliTerm::from_str("-0.2228 * Z1").unwrap())
            .with_term(PauliTerm::from_str("0.1686 * Z0 Z1").unwrap())
            .with_term(PauliTerm::from_str("0.0453 * X0 X1").unwrap());

        let display_str = h2_hamiltonian.to_string();
        println!("H2 Hamiltonian:\n{}", display_str);
        assert!(display_str.contains("-0.8126"));
        assert!(display_str.contains("X0 X1"));
    }
}
