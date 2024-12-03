use super::*;

use bollard::service::{HostConfig, Mount, MountTypeEnum};

/*
This function creates image and container

The role is:
1. Connects to the Docker
2. Creates image
3. Creates container

It accepts the language as an argument.
It returns true in case of success or false or error message otherwise.

Here is an agreement of naming the containers
rustsn_<language>_container
*/

pub fn create_container(lang: &Lang) -> Result<bool, String> {

    if *VERBOSE.lock().unwrap() {
        println!("Creating a container rustsn_{}_container", lang.to_string().to_lowercase());
    }

    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Couldn't connect to Docker: {}", e))?;

    // Create context to running async functions
    // Some Bollard::Docker functions are async
    let rt = Runtime::new().unwrap();
    rt.block_on(async {

        // Set container name in accordance with rules of naming
        let container_name = format!("rustsn_{}_container", lang.to_string().to_lowercase());

        // Set container options - set container name
        let container_options: CreateContainerOptions<String> = CreateContainerOptions {
            name: container_name,
            ..Default::default()
        };

        // Get an absolute path to sandbox directory
        let sandbox_path = std::env::current_dir()
        .map_err(|e| format!("Couldn't get the path to sandbox directory: {}", e))?
        .join("sandbox")
        .to_string_lossy()
        .to_string();

        println!("From fn create_container: sandbox_path: {}", sandbox_path);

        // Set host config
        // Here we bind the sandbox directory to the container
        // At the container the directory is mounted at /app
        // Documentain is: https://docs.rs/bollard/latest/bollard/models/struct.HostConfig.html
        let host_config = HostConfig {
            /*binds: Some(vec![
                //format!("/home/dev/rust/rustsn/src/sandbox:/app:rw")
                //format!("sandbox:/app:rw")
                //format!("{}:/app:rw", sandbox_path)
            ]),*/
            mounts: Some(vec![
                Mount {
                    target: Some("/app".to_string()),  // Путь в контейнере
                    source: Some(sandbox_path),        // Путь на хосте
                    typ: Some(MountTypeEnum::BIND),    // Тип монтирования
                    read_only: Some(false),            // Разрешаем запись
                    ..Default::default()
                }
            ]),
            ..Default::default()
        };

        use std::collections::HashMap;

        // Set volumes
        // Set volumes
        let mut volumes = HashMap::new();
        let mut volume_config = HashMap::new();
        volume_config.insert("bind", "/app");
        volume_config.insert("mode", "rw");

        volumes.insert(
            //&sandbox_path,  // путь на хосте
            "/home/dev/rust/rustsn/src/sandbox",  // путь на хосте
            volume_config   // конфигурация с путём в контейнере и режимом
        );

        // Set container config
        // Here we define the container image, tty, working directory and host config
        // Documentain is: https://docs.rs/bollard/latest/bollard/container/struct.Config.html
        let config = ContainerConfig {
            image: Some(lang.get_image_name()?),
            tty: Some(true),
            working_dir: Some("/app"),
            user: Some("1000:1000"),
            cmd: Some(vec!["/bin/bash"]),
            //host_config: Some(host_config),
            host_config: Some(bollard::service::HostConfig {
                binds: Some(vec![
                    "/home/dev/rust/rustsn/sandbox:/app:rw".to_string()
                ]),
                ..Default::default()
            }),
            //volumes: Some (volumes),
            env: Some(vec![
                "PATH=/usr/local/cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
            ]),
            ..Default::default()
        };

        // Create container
        match docker.create_container(Some(container_options), config).await {
            Ok(container_info) => {
                dbg!(&container_info);
                if *VERBOSE.lock().unwrap() {
                    println!("Container has been created: {:?}", container_info);
                }
                return Ok(true)
            },
            Err(e) => return Err(format!("Couldn't create a container: {}", e))
        };
    })
}

use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::container::LogOutput;
use futures_util::StreamExt;

pub async fn inspect_path () -> Result<(), String> {
        // Подключаемся к Docker
        let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Failed to connect to Docker: {}", e))?;

    // Получаем имя контейнера (предполагаем, что используем Rust контейнер)
    let container_name = "rustsn_rust_container";

    // Создаем конфигурацию для выполнения команды
    let exec_config = CreateExecOptions {
        cmd: Some(vec!["ls", "-la"]),
        attach_stdout: Some(true),
        attach_stderr: Some(true),
        working_dir: Some("/app"),
        user: Some("1000:1000"),
        ..Default::default()
    };

    dbg!("FROM CREATE CONTAINER {:?}",&exec_config);

    // Создаем exec процесс
    let exec = docker.create_exec(container_name, exec_config)
        .await
        .map_err(|e| format!("Failed to create exec: {}", e))?;

    // Запускаем exec процесс
    match docker.start_exec(&exec.id, None).await {
        Ok(StartExecResults::Attached { mut output, .. }) => {
            while let Some(chunk) = output.next().await {
                match chunk {
                    Ok(LogOutput::StdOut { message }) => {
                        println!("Current working directory: {}", String::from_utf8_lossy(&message));
                    }
                    Ok(LogOutput::StdErr { message }) => {
                        eprintln!("Error: {}", String::from_utf8_lossy(&message));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        Ok(StartExecResults::Detached) => {
            Err("Unexpected detached mode".to_string())
        }
        Err(e) => {
            Err(format!("Failed to start exec: {}", e))
        }
    }
}