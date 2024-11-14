// Add these libs to work with a Docker
use bollard::Docker;
use bollard::image::ListImagesOptions;
use bollard::image::CreateImageOptions;
use bollard::container::{CreateContainerOptions, Config as ContainerConfig, StartContainerOptions};
use bollard::service::HostConfig; 
use std::default::Default;

use futures::StreamExt;

use tokio::runtime::Runtime;
use crate::{Lang, VERBOSE};

// Define entity which contains the type of the environment - host or docker
#[allow(non_camel_case_types)]
pub enum EnvironmentType {
    host,
    docker
}

// This function "wrapped in Future".
//
// It connects to a Docker 
// and return the list of existing containers.
//
// This is "async" function cause it's neccessary to
// call async function "list_containers"
// Bollard::Docker lib requires to call async functions
// with Tokio library.

/* Commented as unused function
async fn check_docker_containers (dock: bollard::Docker) -> 
        Result<Vec<bollard::models::ContainerSummary>, String> {

    // Call list of containers and return in to containers
    // Be careful - after calling containers contain Future
    // See below...
    let containers = dock.list_containers(Some(ListContainersOptions::<String>{
        all: true,
        ..Default::default()
    }));

    // Here is function unwraps Future (by calling .await)
    // and after unwrapping there is an opportunity
    // to get the resuls - Vec<bollard::models::ContainerSummary>
    match containers.await {
        Ok(result) => {
            return Ok(result);
        }
        Err(err) => {
            let mut status = String::from ("Couldn't get list of containers. Error: ");
            status.push_str (&err.to_string ());
            return Err(status.to_string());
        }
    }
}
*/

// This function "wrapped in Future".
//
// It connects to a Docker 
// and return the list of existing images.
//
// This is "async" function cause it's neccessary to
// call async function "list_images"
// Bollard::Docker lib requires to call async functions
// with Tokio library.

async fn check_docker_images (dock: bollard::Docker) -> 
        Result<Vec<bollard::models::ImageSummary>, String> {

    // Call list of images and return in to "images"
    // Be careful - after calling containers contain Future
    // See below...
    let images = dock.list_images(Some(ListImagesOptions::<String>{
        all: true,
        ..Default::default()
    }));

    // Here is function unwraps Future (by calling .await)
    // and after unwrapping there is an opportunity
    // to get the resuls - Vec<bollard::models::ContainerSummary>
    match images.await {
        Ok(result) => {
            return Ok(result);
        }
        Err(err) => {
            let mut status = String::from ("Couldn't get list of containers. Error: ");
            status.push_str (&err.to_string ());
            return Err(status.to_string());
        }
    }
}

// This function check the Docker status.
//
// The role is:
//
// 1. Connects to the Docker
// 2. Checks list of images and founds the language specific image there
//
// It returns the status regarding an image  
//
// Here is an agreement of naming the containers
// rustsn_<language>_container
pub fn check_docker (lang: &Lang) -> Result<bool, String>
{
    // Connect to the Docker
    let docker = match Docker::connect_with_socket_defaults() {
        Ok (docker) => docker,
        Err (e) => panic!("Couldn't connect to Docker {:?}", e)
    };

    // Define variable to keeping a list of images
    //let mut images : Vec<bollard::models::ImageSummary> = Vec::new();
    // Run function "check_docker_images" at Tokio context and return the value
    let images = match Runtime::new().unwrap().block_on (check_docker_images(docker.clone ())) {
        Ok (i) => i,
        Err (e) => panic! ("Couldn't get list of images {:?}", e)
    };

    // Processing the list of images
    //
    // The documentaton for the bollard::models::ImageSummary
    // https://docs.rs/bollard/latest/bollard/models/struct.ImageSummary.html
    //
    // Below function checks all names of the containers and compares
    // with the language specific word

    // Its need to find first image and if it found - break
    //let mut image_checker : bool = false;
    // Processing each image with iterator img
    for img in &images {
        // Processing all repo_tags with iterator i
        for i in img.repo_tags.clone () {
            // Variable "i" contains the name of the image
            // If it contains language specific work then OK and break the cicle
            //if i.contains (&format!("rustsn_{}_container", &lang.to_string())) {
                if i.contains (&format!("{}", &lang.to_string())) {
                println! ("Image for {:?} is found", lang);
                //image_checker = true;
                return Ok(true)
            } 
        }
    }

    return Ok(false)

    // At this point the program:
    // 1. Check the connection to the Docker
    // 2. Check that images contain language specific image
    // It returns true if checking is OK
    // Or returns false if doesn't found language specific image


}

// This function creates image and container
//
// It returns true in case of success
// And returns false in case of something goes wrong
//
// Here is an agreement of naming the containers
// rustsn_<language>_container
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
        //let container_options = Some(CreateContainerOptions{
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

        // Set host config
        let host_config = HostConfig {
            binds: Some(vec![
                format!("{}:/app", sandbox_path)
            ]),
            ..Default::default()
        };

        // Set container config
        let config = ContainerConfig {
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
