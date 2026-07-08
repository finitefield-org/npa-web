use std::{error::Error, fmt, path::PathBuf};

use npa_cli::{
    args::{
        PackageAuditCacheMode, PackageBuildCertsOptions, PackageBuildCheckCacheMode,
        PackageChecker, PackageCommonOptions, PackageTimingMode, PackageVerifierMemoMode,
        PackageVerifyCertsOptions,
    },
    diagnostic::{CommandDiagnostic, CommandResult, CommandStatus},
    package_build::run_package_build_certs,
    package_check::run_package_check,
    package_verify::run_package_verify_certs,
};

use crate::render;

const PACKAGE_FIXTURES_ROOT: &str = "../npa-core/testdata/package";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PackageFixtureId {
    #[default]
    NpaStd,
    NpaMathlibSeed,
}

impl PackageFixtureId {
    pub const ALL: [Self; 2] = [Self::NpaStd, Self::NpaMathlibSeed];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NpaStd => "npa-std",
            Self::NpaMathlibSeed => "npa-mathlib-seed",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::NpaStd => "npa-std",
            Self::NpaMathlibSeed => "npa-mathlib-seed",
        }
    }

    const fn fixture_dir(self) -> &'static str {
        match self {
            Self::NpaStd => "npa-std",
            Self::NpaMathlibSeed => "npa-mathlib-seed",
        }
    }

    pub fn from_wire(value: &str) -> Option<Self> {
        match value {
            "npa-std" => Some(Self::NpaStd),
            "npa-mathlib-seed" => Some(Self::NpaMathlibSeed),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFixtureRun {
    pub fixture: PackageFixtureId,
    pub status: String,
    pub root: String,
    pub steps: Vec<PackageFixtureStep>,
    pub diagnostics: Vec<PackageFixtureDiagnostic>,
}

impl PackageFixtureRun {
    pub fn to_view(&self) -> render::PackageFixtureResultView<'_> {
        render::PackageFixtureResultView {
            status: &self.status,
            fixture_label: self.fixture.label(),
            root: &self.root,
            steps: self.steps.iter().map(PackageFixtureStep::to_view).collect(),
            diagnostics: self
                .diagnostics
                .iter()
                .map(PackageFixtureDiagnostic::to_view)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFixtureStep {
    pub command: String,
    pub status: String,
    pub diagnostic_count: usize,
}

impl PackageFixtureStep {
    fn to_view(&self) -> render::PackageFixtureStepView<'_> {
        render::PackageFixtureStepView {
            command: &self.command,
            status: &self.status,
            diagnostic_count: self.diagnostic_count,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackageFixtureDiagnostic {
    pub severity: String,
    pub command: String,
    pub kind: String,
    pub reason: String,
    pub detail: String,
}

impl PackageFixtureDiagnostic {
    fn to_view(&self) -> render::PackageFixtureDiagnosticView<'_> {
        render::PackageFixtureDiagnosticView {
            severity: &self.severity,
            command: &self.command,
            kind: &self.kind,
            reason: &self.reason,
            detail: &self.detail,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageFixtureError {
    message: String,
}

impl PackageFixtureError {
    pub fn unknown_fixture(value: &str) -> Self {
        Self {
            message: format!("Unknown package fixture selection: {value}."),
        }
    }

    pub fn user_message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for PackageFixtureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.message)
    }
}

impl Error for PackageFixtureError {}

pub fn run_package_fixture(fixture: PackageFixtureId) -> PackageFixtureRun {
    let root = package_fixture_root(fixture);
    let common = PackageCommonOptions {
        root: root.clone(),
        json: false,
    };
    let results = [
        run_package_check(common.clone()),
        run_package_build_certs(PackageBuildCertsOptions {
            common: common.clone(),
            check: true,
            build_check_cache: PackageBuildCheckCacheMode::Off,
        }),
        run_package_verify_certs(PackageVerifyCertsOptions {
            common,
            checker: PackageChecker::Fast,
            audit_cache: PackageAuditCacheMode::Off,
            verifier_memo: PackageVerifierMemoMode::Off,
            jobs: 1,
            external: None,
            timings: PackageTimingMode::Off,
        }),
    ];
    let status = if results
        .iter()
        .all(|result| result.status == CommandStatus::Passed)
    {
        "passed"
    } else {
        "failed"
    }
    .to_owned();
    let root = results
        .first()
        .map(|result| result.root.clone())
        .unwrap_or_else(|| root.display().to_string());
    let steps = results
        .iter()
        .map(|result| PackageFixtureStep {
            command: result.command.clone(),
            status: result.status.as_str().to_owned(),
            diagnostic_count: result.diagnostics.len(),
        })
        .collect();
    let diagnostics = results.iter().flat_map(diagnostics_for_result).collect();

    PackageFixtureRun {
        fixture,
        status,
        root,
        steps,
        diagnostics,
    }
}

pub fn package_fixture_root(fixture: PackageFixtureId) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(PACKAGE_FIXTURES_ROOT)
        .join(fixture.fixture_dir())
}

pub fn package_fixture_options(
    selected: PackageFixtureId,
) -> Vec<render::PackageFixtureOptionView<'static>> {
    PackageFixtureId::ALL
        .iter()
        .map(|fixture| render::PackageFixtureOptionView {
            value: fixture.as_str(),
            label: fixture.label(),
            selected: *fixture == selected,
        })
        .collect()
}

pub fn package_fixture_from_wire(value: &str) -> Result<PackageFixtureId, PackageFixtureError> {
    PackageFixtureId::from_wire(value).ok_or_else(|| PackageFixtureError::unknown_fixture(value))
}

fn diagnostics_for_result(result: &CommandResult) -> Vec<PackageFixtureDiagnostic> {
    if result.diagnostics.is_empty() {
        return vec![PackageFixtureDiagnostic {
            severity: "info".to_owned(),
            command: result.command.clone(),
            kind: "Command".to_owned(),
            reason: "command_completed".to_owned(),
            detail: format!(
                "status={};diagnostics=0;proof_evidence=false",
                result.status.as_str()
            ),
        }];
    }

    result
        .diagnostics
        .iter()
        .map(|diagnostic| PackageFixtureDiagnostic {
            severity: diagnostic.severity.as_str().to_owned(),
            command: result.command.clone(),
            kind: diagnostic.kind.as_str().to_owned(),
            reason: diagnostic.reason_code.clone(),
            detail: diagnostic_detail(diagnostic),
        })
        .collect()
}

fn diagnostic_detail(diagnostic: &CommandDiagnostic) -> String {
    let mut fields = Vec::new();
    if let Some(module) = &diagnostic.module {
        fields.push(format!("module={module}"));
    }
    if let Some(path) = &diagnostic.path {
        fields.push(format!("path={path}"));
    }
    if let Some(field) = &diagnostic.field {
        fields.push(format!("field={field}"));
    }
    if let Some(expected) = &diagnostic.expected_value {
        fields.push(format!("expected={expected}"));
    }
    if let Some(actual) = &diagnostic.actual_value {
        fields.push(format!("actual={actual}"));
    }
    if let Some(expected) = &diagnostic.expected_hash {
        fields.push(format!("expected_hash={expected}"));
    }
    if let Some(actual) = &diagnostic.actual_hash {
        fields.push(format!("actual_hash={actual}"));
    }
    if let Some(checker) = &diagnostic.checker {
        fields.push(format!("checker={checker}"));
    }
    if fields.is_empty() {
        "proof_evidence=false".to_owned()
    } else {
        fields.join(";")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_fixture_mode_rejects_unknown_fixture_ids() {
        let error = package_fixture_from_wire("../npa-core/testdata/package/npa-std")
            .expect_err("browser values must be allowlist ids, not paths");

        assert!(error
            .user_message()
            .contains("../npa-core/testdata/package/npa-std"));
    }

    #[test]
    fn package_fixture_mode_runs_fixed_npa_std_workflow() {
        let run = run_package_fixture(PackageFixtureId::NpaStd);
        let commands = run
            .steps
            .iter()
            .map(|step| (step.command.as_str(), step.status.as_str()))
            .collect::<Vec<_>>();

        assert_eq!(run.status, "passed");
        assert_eq!(
            commands,
            vec![
                ("package check", "passed"),
                ("package build-certs", "passed"),
                ("package verify-certs", "passed"),
            ]
        );
        assert!(run
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason == "module_verified"
                && diagnostic.detail.contains("Std.Logic.Eq")));
        assert!(run
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.reason == "module_verified"
                && diagnostic.detail.contains("proof_evidence=true")));
    }
}
