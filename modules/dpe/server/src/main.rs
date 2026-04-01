use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};

mod fragments;

#[derive(Parser)]
#[command(name = "dpe", about = "DaSCH Discovery and Presentation Environment")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the web server
    Serve,
    /// Validate all data files under the given data directory
    Validate {
        /// Path to the data directory containing projects/, persons/, organizations/, records/
        data_dir: PathBuf,
    },
    /// Check if the server is healthy (for Docker HEALTHCHECK)
    Healthcheck {
        #[arg(long, default_value = "http://localhost:8080/healthz")]
        url: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        None => {
            // No subcommand: print help and exit
            use clap::CommandFactory;
            Cli::command().print_help().ok();
            println!();
            ExitCode::SUCCESS
        }
        Some(Commands::Serve) => serve(),
        Some(Commands::Validate { data_dir }) => validate(data_dir),
        Some(Commands::Healthcheck { url }) => healthcheck(&url),
    }
}

#[tokio::main]
async fn serve() -> ExitCode {
    use axum::routing::get;
    use axum::Router;
    use dpe_web::*;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;

    // Generate the list of routes in your Leptos App
    let routes = generate_route_list(App);

    let app = Router::new()
        // OAI-PMH 2.0 endpoint (from dpe-api-oai crate)
        .route("/oai", get(dpe_api_oai::oai_handler))
        // Datastar SSE fragment endpoints
        .route(
            "/projects/{id}/tab/{tab}",
            get(fragments::tab_fragment_handler),
        )
        .route("/projects/search", get(fragments::search_fragment_handler))
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();

    ExitCode::SUCCESS
}

fn validate(data_dir: PathBuf) -> ExitCode {
    use std::fs;

    let mut errors: Vec<String> = Vec::new();
    let mut project_count = 0;
    let mut record_count = 0;
    let mut person_count = 0;
    let mut org_count = 0;

    // Validate projects
    let projects_dir = data_dir.join("projects");
    let mut contributor_ids: Vec<String> = Vec::new();
    if projects_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<dpe_core::ProjectRaw>(&json) {
                        Ok(raw) => {
                            for attr in &raw.attributions {
                                contributor_ids.push(attr.contributor.clone());
                            }
                            if let Some(contacts) = &raw.contact_point {
                                for c in contacts {
                                    contributor_ids.push(c.clone());
                                }
                            }
                            project_count += 1;
                            let _project: dpe_core::Project = raw.into();
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    } else {
        errors.push(format!(
            "projects directory not found: {}",
            projects_dir.display()
        ));
    }

    // Validate records
    let records_dir = data_dir.join("records");
    if records_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&records_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<Vec<dpe_core::Record>>(&json) {
                        Ok(recs) => record_count += recs.len(),
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Validate persons
    let persons_dir = data_dir.join("persons");
    let mut known_person_ids = std::collections::HashSet::new();
    if persons_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&persons_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<dpe_core::Person>(&json) {
                        Ok(p) => {
                            known_person_ids.insert(p.id.clone());
                            person_count += 1;
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Validate organizations
    let orgs_dir = data_dir.join("organizations");
    let mut known_org_ids = std::collections::HashSet::new();
    if orgs_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(&orgs_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let filename = path.display().to_string();
                match fs::read_to_string(&path) {
                    Ok(json) => match serde_json::from_str::<dpe_core::Organization>(&json) {
                        Ok(o) => {
                            known_org_ids.insert(o.id.clone());
                            org_count += 1;
                        }
                        Err(e) => errors.push(format!("{filename}: {e}")),
                    },
                    Err(e) => errors.push(format!("{filename}: {e}")),
                }
            }
        }
    }

    // Cross-reference checks
    for id in &contributor_ids {
        if !known_person_ids.contains(id) && !known_org_ids.contains(id) {
            errors.push(format!(
                "broken reference: contributor '{id}' not found in persons/ or organizations/"
            ));
        }
    }

    // Report results
    println!(
        "Validated: {} projects, {} records, {} persons, {} organizations",
        project_count, record_count, person_count, org_count
    );

    if errors.is_empty() {
        println!("All data files are valid.");
        ExitCode::SUCCESS
    } else {
        println!("\n{} error(s) found:", errors.len());
        for err in &errors {
            println!("  - {err}");
        }
        ExitCode::FAILURE
    }
}

fn healthcheck(url: &str) -> ExitCode {
    match ureq::get(url).call() {
        Ok(response) => {
            if response.status() == 200 {
                ExitCode::SUCCESS
            } else {
                eprintln!("healthcheck: unexpected status {}", response.status());
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("healthcheck: {e}");
            ExitCode::FAILURE
        }
    }
}
