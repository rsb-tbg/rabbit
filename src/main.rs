use axum::{
    extract::Json,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use serde::Deserialize;
use std::{fs, io::Write, path::Path};
use std::{io::Read, process::Command};
use tokio::{fs::File, io::AsyncReadExt};
use tower_http::services::ServeDir;
use zip::{write::SimpleFileOptions, ZipWriter};

#[derive(Deserialize)]
struct ProjectForm {
    project_name: String,
    framework: String,
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let projects_path = format!("../projects/");
    // Create the projects directory if it doesn't exist
    if let Err(error) = fs::create_dir_all(&projects_path) {
        eprintln!("Error creating project path: {}!", error);
        return Err(());
    };

    let app = Router::new()
        .route("/", get(root))
        .nest_service("/static", ServeDir::new("src/static"))
        .route("/create-project", axum::routing::post(create_project))
        .route("/download/:project_name", get(download_project));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

// Serve the HTML form
async fn root() -> impl IntoResponse {
    Html(fs::read_to_string("src/static/index.html").unwrap())
}

// Handle project creation (expecting JSON data)
async fn create_project(Json(form): Json<ProjectForm>) -> impl IntoResponse {
    let project_name = &form.project_name;
    let framework = &form.framework;

    // Create the command to run npm create vite
    let output = Command::new("npm")
        .args(&[
            "create",
            "vite@latest",
            project_name,
            "--",
            "--template",
            framework,
        ])
        .current_dir(&"../projects/".to_string()) // Set the current working directory to the project path
        .output()
        .expect("Failed to create Vite project");

    if !output.status.success() {
        return format!(
            "Failed to create project '{}': {}",
            project_name,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Create a .zip file for the project
    let zip_file_name = format!("/tmp/{}.zip", project_name);
    let project_dir = format!("../projects/{}", project_name);

    if let Err(e) = zip_directory(&Path::new(&project_dir), &zip_file_name) {
        return format!("Failed to zip project '{}': {}", project_name, e);
    }

    // Return a message with the download link
    format!(
        "Project '{}' created successfully. <a href='/download/{}'>Download here</a>.",
        project_name, project_name
    )
}

fn zip_directory(src_dir: &Path, zip_file_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let zip_file = std::fs::File::create(zip_file_name)?;
    let mut zip = ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for entry in walkdir::WalkDir::new(src_dir) {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .strip_prefix(Path::new(src_dir))?
            .to_str()
            .unwrap()
            .to_owned();

        if path.is_file() {
            zip.start_file(name, options)?;
            let mut f = std::fs::File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        } else if !name.is_empty() {
            zip.add_directory(name, options)?;
        }
    }

    zip.finish()?;
    Ok(())
}

async fn download_project(
    axum::extract::Path(project_name): axum::extract::Path<String>,
) -> impl IntoResponse {
    let zip_file_name = format!("/tmp/{}.zip", project_name);
    if let Ok(mut file) = File::open(&zip_file_name).await {
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await.unwrap();

        Response::builder()
            .header("Content-Type", "application/zip")
            .header(
                "Content-Disposition",
                format!("attachment; filename=\"{}.zip\"", project_name),
            )
            .body(contents.into())
            .unwrap()
    } else {
        (axum::http::StatusCode::NOT_FOUND, "File not found").into_response()
    }
}
