use super::*;

use bollard::container::RemoveContainerOptions;

use std::default::Default;

/*
This function removes the container.

The role is:
1. Connects to the Docker
2. Removes the container

It accepts the language as an argument.
It returns true in case of success or false or error message otherwise.

Here is an agreement of naming the containers
rustsn_<language>_container
*/

pub fn remove_container (lang: &Lang) -> Result<bool, String> {

    if *VERBOSE.lock().unwrap() {
        println!("Removing the container");
    }
   // Connect to the Docker
   let docker = Docker::connect_with_local_defaults()
        .map_err(|e| format!("Couldn't connect to Docker: {}", e))?;

    // Create context to running async functions
    // Some Bollard::Docker functions are async
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
    // Set container name in accordance with rules of naming
    let container_name = format!("rustsn_{}_container", lang.to_string().to_lowercase());

    let options = Some(RemoveContainerOptions{
        force: true,
        ..Default::default()
    });
    
    match docker.remove_container (&container_name.to_string(), options).await {
        Ok(_) => {
            return Ok(true)
        },
        Err(e) => {
            return Err(format!("Couldn't remove container: {:?}", e))
        }
    };

    });
    Ok(true)
}