use super::*;

/*
This function runs the container.

The role is:
1. Connects to the Docker
2. Checks that the container is running
3. If not - it starts the container

It accepts the language as an argument.
It returns true in case of success or false or error message otherwise.

Here is an agreement of naming the containers
rustsn_<language>_container
*/

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
                    // If the container is running - return true or error message
                    return Ok(true);
                }
                else {
                    // If the container is not running - start the container
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
