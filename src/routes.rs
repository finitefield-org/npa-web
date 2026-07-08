use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    extract::{Form, Query, State},
    http::{header, StatusCode},
    response::Response,
    routing::{get, post},
    Router,
};
use serde::Deserialize;

use crate::{
    package_fixture::{self, PackageFixtureId},
    render::{self, Renderer},
    state::{CreateSessionInput, DemoMode, LspPanelInput, RunTacticInput, VerifyInput, WebState},
};

const HTMX_MIN_JS: &str = include_str!("../static/vendor/htmx/htmx.min.js");
const HTML_CONTENT_TYPE: &str = "text/html; charset=utf-8";
const TEXT_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
const CSS_CONTENT_TYPE: &str = "text/css; charset=utf-8";
const JAVASCRIPT_CONTENT_TYPE: &str = "text/javascript; charset=utf-8";
pub const DEFAULT_BIND_ADDR: &str = "127.0.0.1:7420";

pub type SharedAppState = Arc<AppState>;

pub struct AppState {
    renderer: Renderer,
    web_state: WebState,
}

impl AppState {
    pub fn new() -> Result<Self, render::RenderError> {
        Ok(Self {
            renderer: Renderer::new()?,
            web_state: WebState::new(),
        })
    }
}

pub fn app() -> Result<Router, render::RenderError> {
    Ok(app_with_state(Arc::new(AppState::new()?)))
}

pub fn app_with_state(state: SharedAppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/demos/select", get(select_demo))
        .route("/package-fixtures/run", post(run_package_fixture))
        .route("/lsp/hover", post(lsp_hover))
        .route("/lsp/completions", post(lsp_completions))
        .route("/lsp/code-actions", post(lsp_code_actions))
        .route("/sessions", post(create_session))
        .route("/tactics/run", post(run_tactic))
        .route("/verify", post(verify))
        .merge(asset_routes())
        .with_state(state)
}

pub fn default_bind_addr() -> SocketAddr {
    DEFAULT_BIND_ADDR
        .parse()
        .expect("default bind address should be valid")
}

pub fn bind_addr_from_args<I, S>(args: I) -> Result<SocketAddr, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let values = args
        .into_iter()
        .skip(1)
        .map(|value| value.as_ref().to_owned())
        .collect::<Vec<_>>();

    match values.as_slice() {
        [] => Ok(default_bind_addr()),
        [addr] => parse_bind_addr(addr),
        [flag, addr] if flag == "--bind" => parse_bind_addr(addr),
        _ => Err("usage: npa-web [--bind HOST:PORT]".to_owned()),
    }
}

pub fn asset_routes<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/assets/htmx.min.js", get(htmx_min_js))
        .route("/assets/app.css", get(app_css))
}

async fn index(State(state): State<SharedAppState>) -> Response {
    let view = render::PageView {
        title: "NPA Web",
        source_form: source_form_view(DemoMode::ImportFree),
        workspace: empty_workspace_view(),
        package_fixture: package_fixture_view(PackageFixtureId::default()),
    };

    render_html(|renderer| renderer.render_page(&view), &state)
}

async fn select_demo(
    State(state): State<SharedAppState>,
    Query(query): Query<SelectDemoQuery>,
) -> Response {
    let demo = match demo_from_wire(query.demo.as_deref()) {
        Ok(demo) => demo,
        Err(message) => return bad_request_response(message),
    };
    let view = source_form_view(demo);

    render_html(|renderer| renderer.render_source_form(&view), &state)
}

async fn create_session(
    State(state): State<SharedAppState>,
    Form(form): Form<CreateSessionForm>,
) -> Response {
    let demo = match demo_from_wire(form.demo.as_deref()) {
        Ok(demo) => demo,
        Err(message) => return render_workspace_error(&state, message),
    };
    let input = CreateSessionInput {
        demo,
        source: form.source,
        module: form.module,
        theorem: form.theorem,
    };

    match state.web_state.create_session(input) {
        Ok(workspace) => {
            let view = workspace.to_view();
            render_html(|renderer| renderer.render_workspace(&view), &state)
        }
        Err(error) => render_workspace_error(&state, error.user_message()),
    }
}

async fn run_package_fixture(
    State(state): State<SharedAppState>,
    Form(form): Form<PackageFixtureForm>,
) -> Response {
    let fixture = match package_fixture::package_fixture_from_wire(&form.fixture) {
        Ok(fixture) => fixture,
        Err(error) => {
            let view = package_fixture_error_view(error.user_message());
            return render_html(
                |renderer| renderer.render_package_fixture_result(&view),
                &state,
            );
        }
    };
    let run = package_fixture::run_package_fixture(fixture);
    let view = run.to_view();

    render_html(
        |renderer| renderer.render_package_fixture_result(&view),
        &state,
    )
}

async fn lsp_hover(
    State(state): State<SharedAppState>,
    Form(form): Form<LspPanelForm>,
) -> Response {
    let input = lsp_input_from_form(form);
    let result = state.web_state.lsp_hover(input);
    let view = result.to_view();

    render_html(|renderer| renderer.render_lsp_hover_result(&view), &state)
}

async fn lsp_completions(
    State(state): State<SharedAppState>,
    Form(form): Form<LspPanelForm>,
) -> Response {
    let input = lsp_input_from_form(form);
    let result = state.web_state.lsp_completions(input);
    let view = result.to_view();

    render_html(
        |renderer| renderer.render_lsp_completion_result(&view),
        &state,
    )
}

async fn lsp_code_actions(
    State(state): State<SharedAppState>,
    Form(form): Form<LspPanelForm>,
) -> Response {
    let input = lsp_input_from_form(form);
    let result = state.web_state.lsp_code_actions(input);
    let view = result.to_view();

    render_html(
        |renderer| renderer.render_lsp_code_action_result(&view),
        &state,
    )
}

async fn run_tactic(
    State(state): State<SharedAppState>,
    Form(form): Form<RunTacticForm>,
) -> Response {
    let input = RunTacticInput {
        session_id: form.session_id.clone(),
        document_id: form.document_id.clone(),
        document_version: form.document_version.clone(),
        state_id: form.state_id.clone(),
        goal_id: form.goal_id.clone(),
        tactic: form.tactic.clone(),
    };

    match state.web_state.run_tactic(input) {
        Ok(workspace) => {
            let view = workspace.to_view();
            render_html(|renderer| renderer.render_workspace(&view), &state)
        }
        Err(error) => render_workspace_form_error(&state, &form, error.user_message()),
    }
}

async fn verify(State(state): State<SharedAppState>, Form(form): Form<VerifyForm>) -> Response {
    let input = VerifyInput {
        session_id: form.session_id,
        document_id: form.document_id,
        document_version: form.document_version,
        state_id: form.state_id,
    };

    match state.web_state.verify(input) {
        Ok(verify) => {
            let view = verify.to_view();
            render_html(|renderer| renderer.render_verify(&view), &state)
        }
        Err(error) => {
            let view = render::VerifyView {
                status: "error",
                detail: error.user_message(),
                root_decl_certificate_hash: "",
                certificate_hash: "",
                imports: Vec::new(),
            };
            render_html(|renderer| renderer.render_verify(&view), &state)
        }
    }
}

pub async fn htmx_min_js() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, JAVASCRIPT_CONTENT_TYPE)
        .body(Body::from(HTMX_MIN_JS))
        .expect("static htmx response should build")
}

pub async fn app_css() -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, CSS_CONTENT_TYPE)
        .body(Body::from(crate::style::app_css()))
        .expect("static app css response should build")
}

fn parse_bind_addr(value: &str) -> Result<SocketAddr, String> {
    value
        .parse()
        .map_err(|_| "bind address must be HOST:PORT".to_owned())
}

fn render_html(
    render: impl FnOnce(&Renderer) -> Result<String, render::RenderError>,
    state: &AppState,
) -> Response {
    match render(&state.renderer) {
        Ok(html) => html_response(StatusCode::OK, html),
        Err(error) => server_error_response(error.user_message()),
    }
}

fn render_workspace_error(state: &AppState, message: &str) -> Response {
    let view = workspace_error_view(message);
    render_html(|renderer| renderer.render_workspace(&view), state)
}

fn render_workspace_form_error(state: &AppState, form: &RunTacticForm, message: &str) -> Response {
    let view = workspace_form_error_view(form, message);
    render_html(|renderer| renderer.render_workspace(&view), state)
}

fn html_response(status: StatusCode, body: String) -> Response {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, HTML_CONTENT_TYPE)
        .body(Body::from(body))
        .expect("html response should build")
}

fn server_error_response(message: &str) -> Response {
    Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, TEXT_CONTENT_TYPE)
        .body(Body::from(message.to_owned()))
        .expect("server error response should build")
}

fn bad_request_response(message: &str) -> Response {
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, TEXT_CONTENT_TYPE)
        .body(Body::from(message.to_owned()))
        .expect("bad request response should build")
}

fn source_form_view(demo: DemoMode) -> render::SourceFormView<'static> {
    render::SourceFormView {
        demos: demo_options(demo),
        source: demo.default_source(),
        module_name: demo.default_module(),
        theorem_name: demo.default_theorem(),
    }
}

fn demo_options(selected: DemoMode) -> Vec<render::DemoOptionView<'static>> {
    DemoMode::ALL
        .iter()
        .map(|demo| render::DemoOptionView {
            value: demo.as_str(),
            label: demo.label(),
            selected: *demo == selected,
        })
        .collect()
}

fn demo_from_wire(value: Option<&str>) -> Result<DemoMode, &'static str> {
    value
        .map(DemoMode::from_wire)
        .unwrap_or(Some(DemoMode::ImportFree))
        .ok_or("Unknown demo selection.")
}

fn package_fixture_view(selected: PackageFixtureId) -> render::PackageFixtureView<'static> {
    render::PackageFixtureView {
        options: package_fixture::package_fixture_options(selected),
        result: pending_package_fixture_result_view(selected),
    }
}

fn pending_package_fixture_result_view(
    fixture: PackageFixtureId,
) -> render::PackageFixtureResultView<'static> {
    render::PackageFixtureResultView {
        status: "not run",
        fixture_label: fixture.label(),
        root: "",
        steps: Vec::new(),
        diagnostics: Vec::new(),
    }
}

fn package_fixture_error_view(message: &str) -> render::PackageFixtureResultView<'_> {
    render::PackageFixtureResultView {
        status: "error",
        fixture_label: "",
        root: "",
        steps: Vec::new(),
        diagnostics: vec![render::PackageFixtureDiagnosticView {
            severity: "error",
            command: "package fixture",
            kind: "Input",
            reason: "unknown_fixture",
            detail: message,
        }],
    }
}

fn lsp_input_from_form(form: LspPanelForm) -> LspPanelInput {
    LspPanelInput {
        session_id: form.session_id.unwrap_or_default(),
        document_id: form.document_id.unwrap_or_default(),
        document_version: form.document_version.unwrap_or_default(),
        state_id: form.state_id.unwrap_or_default(),
        goal_id: form.goal_id.unwrap_or_default(),
        hover_name: form.hover_name.unwrap_or_default(),
    }
}

fn empty_workspace_view<'a>() -> render::WorkspaceView<'a> {
    render::WorkspaceView {
        session_id: "",
        document_id: "",
        document_version: "",
        state_id: "",
        goal_id: "",
        tactic_input: "",
        goal: empty_goal_view(),
        messages: render::MessagesView { items: Vec::new() },
        verify: pending_verify_view(),
        lsp: empty_lsp_panels_view(),
    }
}

fn workspace_error_view(message: &str) -> render::WorkspaceView<'_> {
    render::WorkspaceView {
        messages: error_messages_view(message),
        ..empty_workspace_view()
    }
}

fn workspace_form_error_view<'a>(
    form: &'a RunTacticForm,
    message: &'a str,
) -> render::WorkspaceView<'a> {
    render::WorkspaceView {
        session_id: &form.session_id,
        document_id: &form.document_id,
        document_version: &form.document_version,
        state_id: &form.state_id,
        goal_id: &form.goal_id,
        tactic_input: &form.tactic,
        goal: empty_goal_view(),
        messages: error_messages_view(message),
        verify: pending_verify_view(),
        lsp: lsp_panels_view_from_ids(
            &form.session_id,
            &form.document_id,
            &form.document_version,
            &form.state_id,
            &form.goal_id,
        ),
    }
}

fn empty_goal_view<'a>() -> render::GoalView<'a> {
    render::GoalView {
        has_goal: false,
        label: "",
        context: Vec::new(),
        target: "",
    }
}

fn error_messages_view(message: &str) -> render::MessagesView<'_> {
    render::MessagesView {
        items: vec![render::MessageView {
            severity: "error",
            text: message,
        }],
    }
}

fn pending_verify_view<'a>() -> render::VerifyView<'a> {
    render::VerifyView {
        status: "not run",
        detail: "Verify after all goals are closed.",
        root_decl_certificate_hash: "",
        certificate_hash: "",
        imports: Vec::new(),
    }
}

fn empty_lsp_panels_view<'a>() -> render::LspPanelsView<'a> {
    lsp_panels_view_from_ids("", "", "", "", "")
}

fn lsp_panels_view_from_ids<'a>(
    session_id: &'a str,
    document_id: &'a str,
    document_version: &'a str,
    state_id: &'a str,
    goal_id: &'a str,
) -> render::LspPanelsView<'a> {
    render::LspPanelsView {
        session_id,
        document_id,
        document_version,
        state_id,
        goal_id,
        hover_name: crate::state::DEFAULT_LSP_HOVER_NAME,
        hover: empty_lsp_hover_result_view("not requested"),
        completions: empty_lsp_completion_result_view("not requested"),
        code_actions: empty_lsp_code_action_result_view("not requested"),
    }
}

fn empty_lsp_hover_result_view(status: &str) -> render::LspHoverResultView<'_> {
    render::LspHoverResultView {
        status,
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
    }
}

fn empty_lsp_completion_result_view(status: &str) -> render::LspCompletionResultView<'_> {
    render::LspCompletionResultView {
        status,
        error: "",
        items: Vec::new(),
    }
}

fn empty_lsp_code_action_result_view(status: &str) -> render::LspCodeActionResultView<'_> {
    render::LspCodeActionResultView {
        status,
        error: "",
        actions: Vec::new(),
    }
}

#[derive(Debug, Deserialize)]
struct SelectDemoQuery {
    demo: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateSessionForm {
    demo: Option<String>,
    source: String,
    module: String,
    theorem: String,
}

#[derive(Debug, Deserialize)]
struct PackageFixtureForm {
    fixture: String,
}

#[derive(Debug, Deserialize)]
struct LspPanelForm {
    session_id: Option<String>,
    document_id: Option<String>,
    document_version: Option<String>,
    state_id: Option<String>,
    goal_id: Option<String>,
    hover_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RunTacticForm {
    session_id: String,
    document_id: String,
    document_version: String,
    state_id: String,
    goal_id: String,
    tactic: String,
}

#[derive(Debug, Deserialize)]
struct VerifyForm {
    session_id: String,
    document_id: String,
    document_version: String,
    state_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::to_bytes,
        http::{Method, Request},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn htmx_asset_response_has_javascript_content_type() {
        let response = htmx_min_js().await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            JAVASCRIPT_CONTENT_TYPE
        );
    }

    #[tokio::test]
    async fn htmx_asset_response_serves_vendored_body() {
        let response = htmx_min_js().await;
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("vendored htmx response body should be readable");
        let body = std::str::from_utf8(&body).expect("vendored htmx should be UTF-8");

        assert!(body.starts_with("var htmx=function()"));
        assert!(body.contains("version:\"2.0.9\""));
        assert_eq!(body, HTMX_MIN_JS);
    }

    #[tokio::test]
    async fn app_css_response_has_css_content_type() {
        let response = app_css().await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            CSS_CONTENT_TYPE
        );
    }

    #[tokio::test]
    async fn app_css_response_serves_generated_body() {
        let response = app_css().await;
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("generated app css response body should be readable");
        let body = std::str::from_utf8(&body).expect("generated app css should be UTF-8");

        assert!(body.contains(".npa-theme"));
        assert!(body.contains(".lg\\:grid-cols-2"));
        assert_eq!(body, crate::style::app_css());
    }

    #[test]
    fn asset_router_builds_with_static_routes() {
        let _router = asset_routes::<()>();
    }

    #[test]
    fn routes_default_bind_address_is_localhost() {
        assert_eq!(
            bind_addr_from_args(["npa-web"]).expect("default bind should parse"),
            default_bind_addr()
        );
        assert_eq!(default_bind_addr().to_string(), DEFAULT_BIND_ADDR);
        assert!(default_bind_addr().ip().is_loopback());
    }

    #[test]
    fn routes_explicit_bind_argument_overrides_default() {
        assert_eq!(
            bind_addr_from_args(["npa-web", "--bind", "127.0.0.1:9000"])
                .expect("explicit bind should parse")
                .to_string(),
            "127.0.0.1:9000"
        );
    }

    #[tokio::test]
    async fn routes_index_renders_usable_proof_tool() {
        let response = request(Method::GET, "/", "").await;

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE).unwrap(),
            HTML_CONTENT_TYPE
        );

        let html = response_body(response).await;
        assert!(html.contains("<form id=\"source-panel\""));
        assert!(html.contains("<section id=\"package-fixture-panel\""));
        assert!(html.contains("<section id=\"lsp-panels\""));
        assert!(html.contains("hx-post=\"/sessions\""));
        assert!(html.contains("hx-post=\"/package-fixtures/run\""));
        assert!(html.contains("hx-post=\"/lsp/hover\""));
        assert!(html.contains("hx-post=\"/lsp/completions\""));
        assert!(html.contains("hx-post=\"/lsp/code-actions\""));
        assert!(html.contains("name=\"demo\""));
        assert!(html.contains("value=\"standard\""));
        assert!(html.contains("value=\"npa-std\""));
        assert!(html.contains(crate::state::DEFAULT_SOURCE));
        assert!(!html.contains("landing"));
    }

    #[tokio::test]
    async fn routes_lsp_panels_degrade_without_state() {
        let response = post_form("/lsp/completions", "").await;

        assert_eq!(response.status(), StatusCode::OK);
        let html = response_body(response).await;

        assert!(html.starts_with("\n<section id=\"lsp-completion-result\""));
        assert!(html.contains("No active Human state."));
        assert!(!html.contains("<!doctype html>"));
    }

    #[tokio::test]
    async fn routes_lsp_panels_render_payload_partials() {
        let app = app().expect("routes app should build");
        let workspace = post_form_on(app.clone(), "/sessions", &std_demo_session_body()).await;
        let lsp_body = lsp_body(&workspace, "Eq.refl");

        let hover = post_form_on(app.clone(), "/lsp/hover", &lsp_body).await;
        let completions = post_form_on(app.clone(), "/lsp/completions", &lsp_body).await;
        let code_actions = post_form_on(app, "/lsp/code-actions", &lsp_body).await;

        assert!(hover.starts_with("\n<section id=\"lsp-hover-result\""));
        assert!(hover.contains("Eq.refl"));
        assert!(hover.contains("interface: sha256:"));
        assert!(completions.starts_with("\n<section id=\"lsp-completion-result\""));
        assert!(completions.contains("completion item"));
        assert!(completions.contains("npa.human.search.for_goal"));
        assert!(code_actions.starts_with("\n<section id=\"lsp-code-action-result\""));
        assert!(code_actions.contains("code action"));
        assert!(code_actions.contains("npa.human.search.for_goal"));
        assert!(!hover.contains("<!doctype html>"));
        assert!(!completions.contains("<!doctype html>"));
        assert!(!code_actions.contains("<!doctype html>"));
    }

    #[tokio::test]
    async fn package_fixture_mode_runs_allowlisted_fixture_from_form_id() {
        let response = post_form(
            "/package-fixtures/run",
            &form_body(&[("fixture", PackageFixtureId::NpaStd.as_str())]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let html = response_body(response).await;

        assert!(html.starts_with("\n<section id=\"package-fixture-result\""));
        assert!(html.contains("package check"));
        assert!(html.contains("package build-certs"));
        assert!(html.contains("package verify-certs"));
        assert!(html.contains("module_verified"));
        assert!(html.contains("proof_evidence=true"));
        assert!(!html.contains("<!doctype html>"));
    }

    #[tokio::test]
    async fn package_fixture_mode_rejects_browser_supplied_paths() {
        let response = post_form(
            "/package-fixtures/run",
            &form_body(&[("fixture", "../npa-core/testdata/package/npa-std")]),
        )
        .await;

        assert_eq!(response.status(), StatusCode::OK);
        let html = response_body(response).await;

        assert!(html.contains("unknown_fixture"));
        assert!(html.contains("Unknown package fixture selection"));
        assert!(!html.contains("package verify-certs"));
    }

    #[tokio::test]
    async fn routes_std_demo_selector_returns_standard_source_form() {
        let response = request(Method::GET, "/demos/select?demo=standard", "").await;

        assert_eq!(response.status(), StatusCode::OK);

        let html = response_body(response).await;
        assert!(html.starts_with("\n<form id=\"source-panel\""));
        assert!(html.contains(crate::std_demo::STANDARD_DEMO_SOURCE));
        assert!(html.contains("selected>Standard library</option>"));
        assert!(html.contains("StdDemo.nat_self_eq"));
        assert!(!html.contains("<!doctype html>"));
    }

    #[tokio::test]
    async fn routes_create_session_returns_workspace_partial_with_hidden_ids() {
        let response = post_form("/sessions", &create_session_body()).await;

        assert_eq!(response.status(), StatusCode::OK);
        let html = response_body(response).await;

        assert!(html.starts_with("\n<section id=\"workspace\""));
        assert!(!html.contains("<!doctype html>"));
        assert!(!hidden_value(&html, "session_id").is_empty());
        assert!(!hidden_value(&html, "document_id").is_empty());
        assert_eq!(hidden_value(&html, "document_version"), "1");
        assert!(!hidden_value(&html, "state_id").is_empty());
        assert!(!hidden_value(&html, "goal_id").is_empty());
    }

    #[tokio::test]
    async fn routes_human_flow_completes_default_proof_and_verifies() {
        let app = app().expect("routes app should build");
        let workspace = post_form_on(app.clone(), "/sessions", &create_session_body()).await;

        let workspace = run_tactic_on(app.clone(), &workspace, "intro A").await;
        let workspace = run_tactic_on(app.clone(), &workspace, "intro x").await;
        let workspace = run_tactic_on(app.clone(), &workspace, "exact x").await;

        assert!(workspace.contains("No active goal."));

        let verify = post_form_on(app, "/verify", &verify_body(&workspace)).await;

        assert!(verify.starts_with("\n<section id=\"verify\""));
        assert!(verify.contains("verified"));
        assert!(verify.contains("root declaration: sha256:"));
        assert!(verify.contains("<code class=\"block break-all text-xs\">"));
    }

    #[tokio::test]
    async fn routes_std_demo_flow_completes_and_reports_import_hashes() {
        let app = app().expect("routes app should build");
        let workspace = post_form_on(app.clone(), "/sessions", &std_demo_session_body()).await;

        let workspace = run_tactic_on(app.clone(), &workspace, "intro n").await;
        let workspace = run_tactic_on(app.clone(), &workspace, "exact @Eq.refl.{1} Nat n").await;

        assert!(workspace.contains("No active goal."));

        let verify = post_form_on(app, "/verify", &verify_body(&workspace)).await;

        assert!(verify.contains("verified"));
        assert!(verify.contains("Std.Logic.Eq"));
        assert!(verify.contains("Std.Nat.Basic"));
        assert!(verify.contains("export: sha256:"));
        assert!(verify.contains("certificate: sha256:"));
    }

    #[tokio::test]
    async fn routes_verify_route_returns_only_verify_region() {
        let app = app().expect("routes app should build");
        let workspace = post_form_on(app.clone(), "/sessions", &create_session_body()).await;
        let verify = post_form_on(app, "/verify", &verify_body(&workspace)).await;

        assert!(verify.starts_with("\n<section id=\"verify\""));
        assert!(!verify.contains("id=\"workspace\""));
        assert!(!verify.contains("id=\"source-panel\""));
        assert!(verify.contains("open goals"));
    }

    #[tokio::test]
    async fn routes_error_renders_concise_workspace_message() {
        let body = form_body(&[
            ("module", "Scratch"),
            ("theorem", "Scratch.id"),
            ("source", "import Std.Nat\ntheorem id : Type := Type"),
        ]);
        let response = post_form("/sessions", &body).await;

        assert_eq!(response.status(), StatusCode::OK);
        let html = response_body(response).await;

        assert!(html.contains("Imports are disabled in the import-free demo."));
        assert!(!html.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(!html.contains("panicked"));
    }

    async fn request(method: Method, uri: &str, body: &str) -> Response {
        app()
            .expect("routes app should build")
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .body(Body::from(body.to_owned()))
                    .expect("request should build"),
            )
            .await
            .expect("route request should complete")
    }

    async fn post_form(uri: &str, body: &str) -> Response {
        app()
            .expect("routes app should build")
            .oneshot(form_request(uri, body))
            .await
            .expect("route request should complete")
    }

    async fn post_form_on(app: Router, uri: &str, body: &str) -> String {
        let response = app
            .oneshot(form_request(uri, body))
            .await
            .expect("route request should complete");
        assert_eq!(response.status(), StatusCode::OK);
        response_body(response).await
    }

    async fn run_tactic_on(app: Router, workspace: &str, tactic: &str) -> String {
        post_form_on(app, "/tactics/run", &run_tactic_body(workspace, tactic)).await
    }

    fn form_request(uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(Body::from(body.to_owned()))
            .expect("form request should build")
    }

    async fn response_body(response: Response) -> String {
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("response body should be readable");
        String::from_utf8(body.to_vec()).expect("route body should be UTF-8")
    }

    fn create_session_body() -> String {
        form_body(&[
            ("demo", DemoMode::ImportFree.as_str()),
            ("module", crate::state::DEFAULT_MODULE),
            ("theorem", crate::state::DEFAULT_THEOREM),
            ("source", crate::state::DEFAULT_SOURCE),
        ])
    }

    fn std_demo_session_body() -> String {
        form_body(&[
            ("demo", DemoMode::Standard.as_str()),
            ("module", crate::std_demo::STANDARD_DEMO_MODULE),
            ("theorem", crate::std_demo::STANDARD_DEMO_THEOREM),
            ("source", crate::std_demo::STANDARD_DEMO_SOURCE),
        ])
    }

    fn run_tactic_body(workspace: &str, tactic: &str) -> String {
        form_body(&[
            ("session_id", hidden_value(workspace, "session_id").as_str()),
            (
                "document_id",
                hidden_value(workspace, "document_id").as_str(),
            ),
            (
                "document_version",
                hidden_value(workspace, "document_version").as_str(),
            ),
            ("state_id", hidden_value(workspace, "state_id").as_str()),
            ("goal_id", hidden_value(workspace, "goal_id").as_str()),
            ("tactic", tactic),
        ])
    }

    fn verify_body(workspace: &str) -> String {
        form_body(&[
            ("session_id", hidden_value(workspace, "session_id").as_str()),
            (
                "document_id",
                hidden_value(workspace, "document_id").as_str(),
            ),
            (
                "document_version",
                hidden_value(workspace, "document_version").as_str(),
            ),
            ("state_id", hidden_value(workspace, "state_id").as_str()),
        ])
    }

    fn lsp_body(workspace: &str, hover_name: &str) -> String {
        form_body(&[
            ("session_id", hidden_value(workspace, "session_id").as_str()),
            (
                "document_id",
                hidden_value(workspace, "document_id").as_str(),
            ),
            (
                "document_version",
                hidden_value(workspace, "document_version").as_str(),
            ),
            ("state_id", hidden_value(workspace, "state_id").as_str()),
            ("goal_id", hidden_value(workspace, "goal_id").as_str()),
            ("hover_name", hover_name),
        ])
    }

    fn hidden_value(html: &str, name: &str) -> String {
        let marker = format!("name=\"{name}\" value=\"");
        let start = html
            .find(&marker)
            .unwrap_or_else(|| panic!("missing hidden field {name}"))
            + marker.len();
        let rest = &html[start..];
        let end = rest
            .find('"')
            .unwrap_or_else(|| panic!("unterminated hidden field {name}"));
        rest[..end].to_owned()
    }

    fn form_body(pairs: &[(&str, &str)]) -> String {
        pairs
            .iter()
            .map(|(key, value)| format!("{}={}", encode_component(key), encode_component(value)))
            .collect::<Vec<_>>()
            .join("&")
    }

    fn encode_component(value: &str) -> String {
        let mut encoded = String::new();
        for byte in value.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                b' ' => encoded.push('+'),
                _ => encoded.push_str(&format!("%{byte:02X}")),
            }
        }
        encoded
    }
}
