// Add these libs to work with a Docker
use bollard::Docker;
use bollard::container::ListContainersOptions;
use bollard::image::ListImagesOptions;
use std::collections::HashMap;
use std::default::Default;

use tokio::runtime::Runtime;



// Define entity which contains the type of the environment - host or docker
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
// 2. Checks list of containers and founds the Ollama there
// 3. Checks list of images and founds the Ollama there
//
// If any of conditions isn't completed, 
// it ends and prints the status 
//
// Note. Add expanded information to helping user to solve problems.
// Like if system couldn't connect to the Docker - how to 
// check the status in command line
pub fn check_docker () 
{
    // Connect to the Docker
    let docker = match Docker::connect_with_socket_defaults() {
        Ok (docker) => docker,
        Err (e) => panic!("Couldn't connect to Docker {:?}", e)
    };

    // Define variable to keeping a list of containers
    let mut containers : Vec<bollard::models::ContainerSummary> = Vec::new();
    // Run function "check_docker_containers" at Tokio context and return the value
    containers = match Runtime::new().unwrap().block_on (check_docker_containers(docker.clone ())) {
        Ok (c) => c,
        Err (e) => panic! ("Couldn't get list of containers {:?}", e)
    };

    // Processing the list of containers
    //
    // The documentaton for the bollard::models::ContainerSummary
    // https://docs.rs/bollard/latest/bollard/models/struct.ContainerSummary.html
    //
    // Below function checks all names of the containers and compares
    // with the word "ollama"

    // Its need to find first container and if it found - break
    let mut ollama_checker : bool = false;
    // Processing each container with iterator cont
    for cont in &containers {
        // Processing all names with iterator i
        for mut i in cont.names.clone () {
            // Variable "name" contains the name of the container
            let mut name = i.pop ().expect("Unknown error at procedure of parsing list of Docker containers").clone ();
            // If it contains "ollama" then OK and break the cicle
            if name.contains ("ollama") {
                println! ("Ollama container is found");
                ollama_checker = true;
                break;
            } 
        }
        if ollama_checker == true {
            break;
        }        
    }
    if ollama_checker == false {
        println! ("Ollama container is not found");
        println! ("Please read the documentation");
        panic! ();
    }


    // Define variable to keeping a list of images
    let mut images : Vec<bollard::models::ImageSummary> = Vec::new();
    // Run function "check_docker_images" at Tokio context and return the value
    images = match Runtime::new().unwrap().block_on (check_docker_images(docker.clone ())) {
        Ok (i) => i,
        Err (e) => panic! ("Couldn't get list of images {:?}", e)
    };

    // Processing the list of images
    //
    // The documentaton for the bollard::models::ImageSummary
    // https://docs.rs/bollard/latest/bollard/models/struct.ImageSummary.html
    //
    // Below function checks all names of the containers and compares
    // with the word "ollama"

    // Its need to find first image and if it found - break
    let mut ollama_checker : bool = false;
    // Processing each image with iterator img
    for img in &images {
        // Processing all repo_tags with iterator i
        for mut i in img.repo_tags.clone () {
            // Variable "i" contains the name of the image
            //let mut name = i.pop ().expect("Unknown error at procedure of parsing list of Docker images").clone ();
            // If it contains "ollama" then OK and break the cicle
            if i.contains ("ollama") {
                println! ("Ollama image is found");
                ollama_checker = true;
                break;
            } 
        }
        if ollama_checker == true {
            break;
        }         
    }
    if ollama_checker == false {
        println! ("Ollama image is not found");
        println! ("Please read the documentation");
        panic! ();
    }

    // At this point the program:
    // 1. Check the connection to the Docker
    // 2. Check that container contain "ollama"
    // 3. Check that images contain "ollama"
    // The checking is complete

    println!("Checking the Docker has done");

}
