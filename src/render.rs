use std::{error::Error, fmt};

use go_html_template::{Template, TemplateError};
use serde::Serialize;

const PAGE_TEMPLATE: &str = include_str!("../templates/page.html");
const LSP_TEMPLATE: &str = include_str!("../templates/lsp.html");
const PACKAGE_FIXTURE_TEMPLATE: &str = include_str!("../templates/package_fixture.html");
const SOURCE_FORM_TEMPLATE: &str = include_str!("../templates/source_form.html");
const WORKSPACE_TEMPLATE: &str = include_str!("../templates/workspace.html");
const GOAL_TEMPLATE: &str = include_str!("../templates/goal.html");
const MESSAGES_TEMPLATE: &str = include_str!("../templates/messages.html");
const VERIFY_TEMPLATE: &str = include_str!("../templates/verify.html");

pub(crate) const TEMPLATE_SOURCES: &[&str] = &[
    PAGE_TEMPLATE,
    LSP_TEMPLATE,
    PACKAGE_FIXTURE_TEMPLATE,
    SOURCE_FORM_TEMPLATE,
    WORKSPACE_TEMPLATE,
    GOAL_TEMPLATE,
    MESSAGES_TEMPLATE,
    VERIFY_TEMPLATE,
];

pub struct Renderer {
    template: Template,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateName {
    Page,
    LspPanels,
    LspHoverResult,
    LspCompletionResult,
    LspCodeActionResult,
    PackageFixture,
    PackageFixtureResult,
    SourceForm,
    Workspace,
    Goal,
    Messages,
    Verify,
}

impl TemplateName {
    fn as_str(self) -> &'static str {
        match self {
            TemplateName::Page => "page",
            TemplateName::LspPanels => "lsp_panels",
            TemplateName::LspHoverResult => "lsp_hover_result",
            TemplateName::LspCompletionResult => "lsp_completion_result",
            TemplateName::LspCodeActionResult => "lsp_code_action_result",
            TemplateName::PackageFixture => "package_fixture",
            TemplateName::PackageFixtureResult => "package_fixture_result",
            TemplateName::SourceForm => "source_form",
            TemplateName::Workspace => "workspace",
            TemplateName::Goal => "goal",
            TemplateName::Messages => "messages",
            TemplateName::Verify => "verify",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderError {
    template: &'static str,
    phase: RenderErrorPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderErrorPhase {
    Parse,
    Execute,
}

impl RenderError {
    fn parse(template: &'static str, _source: TemplateError) -> Self {
        Self {
            template,
            phase: RenderErrorPhase::Parse,
        }
    }

    fn execute(template: &'static str, _source: TemplateError) -> Self {
        Self {
            template,
            phase: RenderErrorPhase::Execute,
        }
    }

    pub fn user_message(&self) -> &'static str {
        match self.phase {
            RenderErrorPhase::Parse => "template parse failed",
            RenderErrorPhase::Execute => "template render failed",
        }
    }

    pub fn template(&self) -> &'static str {
        self.template
    }
}

impl fmt::Display for RenderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.template, self.user_message())
    }
}

impl Error for RenderError {}

impl Renderer {
    pub fn new() -> Result<Self, RenderError> {
        Self::from_source(&template_bundle())
    }

    fn from_source(source: &str) -> Result<Self, RenderError> {
        let template = Template::new(TemplateName::Page.as_str())
            .option("missingkey=error")
            .map_err(|error| RenderError::parse("templates", error))?
            .parse(source)
            .map_err(|error| RenderError::parse("templates", error))?;
        Ok(Self { template })
    }

    pub fn render_page(&self, view: &PageView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::Page, view)
    }

    pub fn render_lsp_panels(&self, view: &LspPanelsView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::LspPanels, view)
    }

    pub fn render_lsp_hover_result(
        &self,
        view: &LspHoverResultView<'_>,
    ) -> Result<String, RenderError> {
        self.render(TemplateName::LspHoverResult, view)
    }

    pub fn render_lsp_completion_result(
        &self,
        view: &LspCompletionResultView<'_>,
    ) -> Result<String, RenderError> {
        self.render(TemplateName::LspCompletionResult, view)
    }

    pub fn render_lsp_code_action_result(
        &self,
        view: &LspCodeActionResultView<'_>,
    ) -> Result<String, RenderError> {
        self.render(TemplateName::LspCodeActionResult, view)
    }

    pub fn render_package_fixture(
        &self,
        view: &PackageFixtureView<'_>,
    ) -> Result<String, RenderError> {
        self.render(TemplateName::PackageFixture, view)
    }

    pub fn render_package_fixture_result(
        &self,
        view: &PackageFixtureResultView<'_>,
    ) -> Result<String, RenderError> {
        self.render(TemplateName::PackageFixtureResult, view)
    }

    pub fn render_source_form(&self, view: &SourceFormView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::SourceForm, view)
    }

    pub fn render_workspace(&self, view: &WorkspaceView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::Workspace, view)
    }

    pub fn render_goal(&self, view: &GoalView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::Goal, view)
    }

    pub fn render_messages(&self, view: &MessagesView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::Messages, view)
    }

    pub fn render_verify(&self, view: &VerifyView<'_>) -> Result<String, RenderError> {
        self.render(TemplateName::Verify, view)
    }

    fn render<T: Serialize>(&self, name: TemplateName, view: &T) -> Result<String, RenderError> {
        self.template
            .execute_template_to_string(name.as_str(), view)
            .map_err(|error| RenderError::execute(name.as_str(), error))
    }
}

fn template_bundle() -> String {
    TEMPLATE_SOURCES.join("\n")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PageView<'a> {
    pub title: &'a str,
    pub source_form: SourceFormView<'a>,
    pub workspace: WorkspaceView<'a>,
    pub package_fixture: PackageFixtureView<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PackageFixtureView<'a> {
    pub options: Vec<PackageFixtureOptionView<'a>>,
    pub result: PackageFixtureResultView<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PackageFixtureOptionView<'a> {
    pub value: &'a str,
    pub label: &'a str,
    pub selected: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PackageFixtureResultView<'a> {
    pub status: &'a str,
    pub fixture_label: &'a str,
    pub root: &'a str,
    pub steps: Vec<PackageFixtureStepView<'a>>,
    pub diagnostics: Vec<PackageFixtureDiagnosticView<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PackageFixtureStepView<'a> {
    pub command: &'a str,
    pub status: &'a str,
    pub diagnostic_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct PackageFixtureDiagnosticView<'a> {
    pub severity: &'a str,
    pub command: &'a str,
    pub kind: &'a str,
    pub reason: &'a str,
    pub detail: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SourceFormView<'a> {
    pub demos: Vec<DemoOptionView<'a>>,
    pub source: &'a str,
    pub module_name: &'a str,
    pub theorem_name: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DemoOptionView<'a> {
    pub value: &'a str,
    pub label: &'a str,
    pub selected: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct WorkspaceView<'a> {
    pub session_id: &'a str,
    pub document_id: &'a str,
    pub document_version: &'a str,
    pub state_id: &'a str,
    pub goal_id: &'a str,
    pub tactic_input: &'a str,
    pub goal: GoalView<'a>,
    pub messages: MessagesView<'a>,
    pub verify: VerifyView<'a>,
    pub lsp: LspPanelsView<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LspPanelsView<'a> {
    pub session_id: &'a str,
    pub document_id: &'a str,
    pub document_version: &'a str,
    pub state_id: &'a str,
    pub goal_id: &'a str,
    pub hover_name: &'a str,
    pub hover: LspHoverResultView<'a>,
    pub completions: LspCompletionResultView<'a>,
    pub code_actions: LspCodeActionResultView<'a>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LspHoverResultView<'a> {
    pub status: &'a str,
    pub contents: &'a str,
    pub theorem_name: &'a str,
    pub module: &'a str,
    pub kind: &'a str,
    pub statement: &'a str,
    pub attributes: &'a str,
    pub axioms: &'a str,
    pub export_hash: &'a str,
    pub certificate_hash: &'a str,
    pub decl_interface_hash: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LspCompletionResultView<'a> {
    pub status: &'a str,
    pub error: &'a str,
    pub items: Vec<LspCompletionItemView<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LspCompletionItemView<'a> {
    pub label: &'a str,
    pub kind: &'a str,
    pub detail: &'a str,
    pub insert_text: &'a str,
    pub command: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LspCodeActionResultView<'a> {
    pub status: &'a str,
    pub error: &'a str,
    pub actions: Vec<LspCodeActionView<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LspCodeActionView<'a> {
    pub title: &'a str,
    pub kind: &'a str,
    pub tactic: &'a str,
    pub command: &'a str,
    pub diagnostic_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct GoalView<'a> {
    pub has_goal: bool,
    pub label: &'a str,
    pub context: Vec<BindingView<'a>>,
    pub target: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct BindingView<'a> {
    pub name: &'a str,
    pub ty: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MessagesView<'a> {
    pub items: Vec<MessageView<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MessageView<'a> {
    pub severity: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct VerifyView<'a> {
    pub status: &'a str,
    pub detail: &'a str,
    pub root_decl_certificate_hash: &'a str,
    pub certificate_hash: &'a str,
    pub imports: Vec<VerifyImportView<'a>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct VerifyImportView<'a> {
    pub module: &'a str,
    pub export_hash: &'a str,
    pub certificate_hash: &'a str,
    pub axiom_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn render_page_escapes_user_source_textarea_content() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = sample_page_view("theorem bad : Type := <tag> & \"quote\"", "", "");

        let html = renderer
            .render_page(&view)
            .expect("page should render with escaped source");

        assert!(html.contains("theorem bad : Type := &lt;tag&gt; &amp; &#34;quote&#34;"));
        assert!(!html.contains("<tag> & \"quote\""));
    }

    #[test]
    fn render_page_neutralizes_textarea_breakout_source() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = sample_page_view("</textarea><script>alert(1)</script>", "", "");

        let html = renderer
            .render_page(&view)
            .expect("page should render with neutralized source");

        assert!(!html.contains("</textarea><script>"));
        assert!(!html.contains("<script>alert(1)</script>"));
    }

    #[test]
    fn render_workspace_escapes_tactic_input_attribute_content() {
        let renderer = Renderer::new().expect("renderer should parse");
        let mut workspace = sample_workspace_view();
        workspace.tactic_input = "\" autofocus onfocus=\"alert(1)";

        let html = renderer
            .render_workspace(&workspace)
            .expect("workspace should render with escaped tactic input");

        assert!(html.contains("value=\"&#34; autofocus onfocus=&#34;alert(1)\""));
        assert!(!html.contains("autofocus onfocus=\"alert(1)"));
    }

    #[test]
    fn render_messages_escapes_diagnostics() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = MessagesView {
            items: vec![MessageView {
                severity: "error",
                text: "<b>bad tactic</b> & retry",
            }],
        };

        let html = renderer
            .render_messages(&view)
            .expect("messages should render with escaped diagnostics");

        assert!(html.contains("&lt;b&gt;bad tactic&lt;/b&gt; &amp; retry"));
        assert!(!html.contains("<b>bad tactic</b>"));
    }

    #[test]
    fn render_package_fixture_result_escapes_diagnostics() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = PackageFixtureResultView {
            status: "passed <ok>",
            fixture_label: "fixture<script>",
            root: "root & path",
            steps: vec![PackageFixtureStepView {
                command: "package verify-certs",
                status: "passed",
                diagnostic_count: 1,
            }],
            diagnostics: vec![PackageFixtureDiagnosticView {
                severity: "info",
                command: "package verify-certs",
                kind: "FastVerifier",
                reason: "module_verified",
                detail: "module=<bad> & proof_evidence=true",
            }],
        };

        let html = renderer
            .render_package_fixture_result(&view)
            .expect("package result should render");

        assert!(html.contains("passed &lt;ok&gt;"));
        assert!(html.contains("fixture&lt;script&gt;"));
        assert!(html.contains("root &amp; path"));
        assert!(html.contains("Diagnostics (untrusted metadata)"));
        assert!(html.contains("module=&lt;bad&gt; &amp; proof_evidence=true"));
        assert!(!html.contains("<bad>"));
    }

    #[test]
    fn render_lsp_panels_escape_payload_metadata() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = LspPanelsView {
            session_id: "sess_1",
            document_id: "doc_1",
            document_version: "1",
            state_id: "state_1",
            goal_id: "goal_1",
            hover_name: "Bad.<script>",
            hover: LspHoverResultView {
                status: "found",
                contents: "```npa\nBad : <bad>\n```",
                theorem_name: "Bad.<script>",
                module: "Bad",
                kind: "theorem",
                statement: "A & B",
                attributes: "simp",
                axioms: "none",
                export_hash: "sha256:<export>",
                certificate_hash: "sha256:<cert>",
                decl_interface_hash: "sha256:<iface>",
            },
            completions: LspCompletionResultView {
                status: "1 completion item",
                error: "",
                items: vec![LspCompletionItemView {
                    label: "exact <bad>",
                    kind: "tactic",
                    detail: "uses &",
                    insert_text: "exact <bad>",
                    command: "",
                }],
            },
            code_actions: LspCodeActionResultView {
                status: "1 code action",
                error: "",
                actions: vec![LspCodeActionView {
                    title: "Run <bad>",
                    kind: "quickfix",
                    tactic: "exact <bad>",
                    command: "",
                    diagnostic_count: 0,
                }],
            },
        };

        let html = renderer
            .render_lsp_panels(&view)
            .expect("LSP panels should render");

        assert!(html.contains("Bad.&lt;script&gt;"));
        assert!(html.contains("A &amp; B"));
        assert!(html.contains("sha256:&lt;cert&gt;"));
        assert!(html.contains("exact &lt;bad&gt;"));
        assert!(!html.contains("<script>"));
        assert!(!html.contains("exact <bad>"));
    }

    #[test]
    fn render_goal_escapes_context_and_target() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = GoalView {
            has_goal: true,
            label: "goal <1>",
            context: vec![BindingView {
                name: "x<script>",
                ty: "A & B",
            }],
            target: "</section>",
        };

        let html = renderer
            .render_goal(&view)
            .expect("goal should render with escaped values");

        assert!(html.contains("goal &lt;1&gt;"));
        assert!(html.contains("x&lt;script&gt;"));
        assert!(html.contains("A &amp; B"));
        assert!(html.contains("&lt;/section&gt;"));
    }

    #[test]
    fn render_verify_escapes_certificate_display_fields() {
        let renderer = Renderer::new().expect("renderer should parse");
        let view = VerifyView {
            status: "verified <ok>",
            detail: "hash & imports",
            root_decl_certificate_hash: "sha256:<root>",
            certificate_hash: "sha256:<bad>",
            imports: vec![VerifyImportView {
                module: "Std.<bad>",
                export_hash: "sha256:<export>",
                certificate_hash: "sha256:<cert>",
                axiom_count: 1,
            }],
        };

        let html = renderer
            .render_verify(&view)
            .expect("verify should render with escaped fields");

        assert!(html.contains("verified &lt;ok&gt;"));
        assert!(html.contains("hash &amp; imports"));
        assert!(html.contains("sha256:&lt;root&gt;"));
        assert!(html.contains("sha256:&lt;bad&gt;"));
        assert!(html.contains("Std.&lt;bad&gt;"));
        assert!(html.contains("sha256:&lt;export&gt;"));
        assert!(html.contains("sha256:&lt;cert&gt;"));
    }

    #[test]
    fn render_error_message_is_short_and_sanitized() {
        let renderer = Renderer::from_source(r#"{{define "page"}}{{.Missing}}{{end}}"#)
            .expect("test template should parse");

        let error = renderer
            .render(TemplateName::Page, &json!({}))
            .expect_err("missing key should be converted");

        assert_eq!(error.user_message(), "template render failed");
        assert_eq!(error.to_string(), "page: template render failed");
        let formatted = error.to_string();
        assert!(!formatted.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(!formatted.contains("panicked"));
        assert!(!formatted.contains("Missing"));
    }

    #[test]
    fn parse_error_message_is_short_and_sanitized() {
        let error = match Renderer::from_source(r#"{{define "page"}}{{if .Open}}"#) {
            Ok(_) => panic!("bad template should fail during parsing"),
            Err(error) => error,
        };

        assert_eq!(error.user_message(), "template parse failed");
        let formatted = error.to_string();
        assert!(!formatted.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(!formatted.contains("panicked"));
        assert!(!formatted.contains("Open"));
    }

    fn sample_page_view<'a>(
        source: &'a str,
        tactic_input: &'a str,
        diagnostic: &'a str,
    ) -> PageView<'a> {
        PageView {
            title: "NPA Web",
            source_form: SourceFormView {
                demos: vec![DemoOptionView {
                    value: "import-free",
                    label: "Import-free",
                    selected: true,
                }],
                source,
                module_name: "Scratch",
                theorem_name: "Scratch.id",
            },
            workspace: WorkspaceView {
                tactic_input,
                messages: MessagesView {
                    items: if diagnostic.is_empty() {
                        Vec::new()
                    } else {
                        vec![MessageView {
                            severity: "info",
                            text: diagnostic,
                        }]
                    },
                },
                ..sample_workspace_view()
            },
            package_fixture: PackageFixtureView {
                options: vec![PackageFixtureOptionView {
                    value: "npa-std",
                    label: "npa-std",
                    selected: true,
                }],
                result: PackageFixtureResultView {
                    status: "not run",
                    fixture_label: "npa-std",
                    root: "",
                    steps: Vec::new(),
                    diagnostics: Vec::new(),
                },
            },
        }
    }

    fn sample_workspace_view<'a>() -> WorkspaceView<'a> {
        WorkspaceView {
            session_id: "sess_1",
            document_id: "doc_1",
            document_version: "1",
            state_id: "state_1",
            goal_id: "goal_1",
            tactic_input: "intro A",
            goal: GoalView {
                has_goal: true,
                label: "goal_1",
                context: vec![BindingView {
                    name: "A",
                    ty: "Type",
                }],
                target: "A",
            },
            messages: MessagesView { items: Vec::new() },
            verify: VerifyView {
                status: "not verified",
                detail: "Run verify after closing all goals.",
                root_decl_certificate_hash: "",
                certificate_hash: "",
                imports: Vec::new(),
            },
            lsp: sample_lsp_panels_view(),
        }
    }

    fn sample_lsp_panels_view<'a>() -> LspPanelsView<'a> {
        LspPanelsView {
            session_id: "sess_1",
            document_id: "doc_1",
            document_version: "1",
            state_id: "state_1",
            goal_id: "goal_1",
            hover_name: "Scratch.id",
            hover: LspHoverResultView {
                status: "not requested",
                contents: "",
                theorem_name: "",
                module: "",
                kind: "",
                statement: "",
                attributes: "",
                axioms: "",
                export_hash: "",
                certificate_hash: "",
                decl_interface_hash: "",
            },
            completions: LspCompletionResultView {
                status: "not requested",
                error: "",
                items: Vec::new(),
            },
            code_actions: LspCodeActionResultView {
                status: "not requested",
                error: "",
                actions: Vec::new(),
            },
        }
    }
}
