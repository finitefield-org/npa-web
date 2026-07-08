use std::{error::Error, fmt};

use npa_cert::{AxiomPolicy, Name, VerifiedModule, VerifierSession};

const STD_LOGIC_EQ_CERT: &[u8] =
    include_bytes!("../../npa-core/testdata/package/npa-std/Std/Logic/Eq/certificate.npcert");
const STD_NAT_BASIC_CERT: &[u8] =
    include_bytes!("../../npa-core/testdata/package/npa-std/Std/Nat/Basic/certificate.npcert");

pub const STANDARD_DEMO_SOURCE: &str = "\
import Std.Nat.Basic
import Std.Logic.Eq

theorem nat_self_eq (n : Nat) : Eq.{1} Nat n n := by
  intro n
  exact @Eq.refl.{1} Nat n";
pub const STANDARD_DEMO_MODULE: &str = "StdDemo";
pub const STANDARD_DEMO_THEOREM: &str = "StdDemo.nat_self_eq";
pub const STANDARD_DEMO_IMPORTS: &[&str] = &["Std.Nat.Basic", "Std.Logic.Eq"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StdDemoLoadError {
    message: String,
}

impl StdDemoLoadError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn user_message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for StdDemoLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for StdDemoLoadError {}

pub fn load_standard_demo_verified_modules() -> Result<Vec<VerifiedModule>, StdDemoLoadError> {
    let mut session = VerifierSession::new();
    let nat = verify_fixture_module("Std.Nat.Basic", STD_NAT_BASIC_CERT, &mut session)?;
    let eq = verify_fixture_module("Std.Logic.Eq", STD_LOGIC_EQ_CERT, &mut session)?;

    Ok(vec![nat, eq])
}

fn verify_fixture_module(
    expected_module: &str,
    bytes: &[u8],
    session: &mut VerifierSession,
) -> Result<VerifiedModule, StdDemoLoadError> {
    let verified =
        npa_cert::verify_module_cert(bytes, session, &AxiomPolicy::normal()).map_err(|error| {
            StdDemoLoadError::new(format!(
                "standard demo fixture {expected_module} failed certificate verification: {error:?}"
            ))
        })?;
    let expected = Name::from_dotted(expected_module);
    if verified.module() != &expected {
        return Err(StdDemoLoadError::new(format!(
            "standard demo fixture expected {expected_module} but loaded {}.",
            verified.module().as_dotted()
        )));
    }
    Ok(verified)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn std_demo_loads_fixed_verified_modules_from_embedded_fixtures() {
        let modules = load_standard_demo_verified_modules()
            .expect("embedded standard demo fixtures should verify");
        let names = modules
            .iter()
            .map(|module| module.module().as_dotted())
            .collect::<Vec<_>>();

        assert_eq!(names, vec!["Std.Nat.Basic", "Std.Logic.Eq"]);
        assert!(modules
            .iter()
            .all(|module| !module.export_block().is_empty()));
        assert!(modules
            .iter()
            .all(|module| module.certificate_hash() != [0; 32]));
    }
}
