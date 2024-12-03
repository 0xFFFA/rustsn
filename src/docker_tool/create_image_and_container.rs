use super::*;

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

pub fn create_image_and_container(lang: &Lang) -> Result<bool, String> {
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Couldn't connect to Docker: {}", e))?;

    // Create context to running async functions
    // Some Bollard::Docker functions are async
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Set image options using language specific image name
        // The image name is defined in Lang::get_image_name()
        // Lang::get_image_name() defined at main.rs
        let image_options = Some(CreateImageOptions{
            from_image: lang.get_image_name()?,
            ..Default::default()
        });
        // Create stream to get the result of creating image
        let mut stream = docker.create_image(image_options, None, None);
        // Process the stream
        while let Some(result) = stream.next().await {
            match result {
                Ok(output) => {
                    if *VERBOSE.lock().unwrap() {
                        println!("Create an image: {:?}", output);
                    }
                },
                Err(e) => return Err(format!("Couldn't create an image: {}", e))
            }
        }

        // Set container name in accordance with rules of naming
        let container_name = format!("rustsn_{}_container", lang.to_string().to_lowercase());

        // Set container options
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

        println!("From fn create_image_and_container: sandbox_path: {}", sandbox_path);

        // Set host config
        // Here we bind the sandbox directory to the container
        // At the container the directory is mounted at /app
        let host_config = HostConfig {
            binds: Some(vec![
                format!("{}:/app", sandbox_path)
            ]),
            ..Default::default()
        };

        // Set container config
        // Here we define the container image, tty, working directory and host config
        let config = ContainerConfig {
            //user: Some("dev"),
            image: Some(lang.get_image_name()?),
            tty: Some(true),
            working_dir: Some("/app"),
            host_config: Some(host_config),
            ..Default::default()
        };

        // Create container
        match docker.create_container(Some(container_options), config).await {
            Ok(container_info) => {
                if *VERBOSE.lock().unwrap() {
                    println!("Container has been created: {:?}", container_info);
                }
                return Ok(true)
            },
            Err(e) => return Err(format!("Couldn't create a container: {}", e))
        };
    })
}