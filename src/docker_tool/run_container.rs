use super::*;

// This function run the container
//
// It returns true in case of success
// And returns false in case of something goes wrong
//
// Initially it checks that the container is running
// If not - it starts the container
//
// Here is an agreement of naming the containers
// rustsn_<language>_container
pub fn run_container (lang: &Lang) -> Result<bool, String> {
    
    // Connect to the Docker
    let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Couldn't connect to Docker: {}", e))?;

    // Create context to running async functions
    // Some Bollard::Docker functions are async
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Set container name in accordance with rules of naming
        let container_name = format!("rustsn_{}_container", lang.to_string().to_lowercase());
        
        // Check that the container is running
        match docker.inspect_container(&container_name, None).await {
            Ok(container_info) => {
                if container_info.state.unwrap().running.expect("Couldn't check the container state") {
                    return Ok(true);
                }
                else {
                    // Start the container
                    match docker.start_container(&container_name, None::<StartContainerOptions<String>>).await {
                        Ok(_) => {
                            if *VERBOSE.lock().unwrap() {
                                println!("Container is {} running", container_name);
                            }
                            return Ok(true)
                        },
                        Err(e) => return Err(format!("Couldn't start a container {}: {}", container_name, e))
                    };     
                }
            }
            Err(e) => return Err(format!("Couldn't check the container {}: {}", container_name, e))
        };
    })
}
