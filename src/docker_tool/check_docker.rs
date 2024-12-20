use super::*;
use super::check_docker_images::check_docker_images;

/*
This function check the Docker status.

The role is:
1. Connects to the Docker
2. Checks list of images and founds the language specific image there

It accepts the language as an argument.
It returns true if OK (Docker is running and language specific image exists) or false or error message otherwise.

Here is an agreement of naming the containers
rustsn_<language>_container

*/

pub fn check_docker (lang: &Lang) -> Result<bool, String>
{
    // Connect to the Docker
    let docker = match Docker::connect_with_socket_defaults() {
        Ok (docker) => docker,
        Err (e) => panic!("Couldn't connect to Docker {:?}", e)
    };

    // Run function "check_docker_images" at Tokio context and return the value to the "images" variable
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
    // with the language specific name 

    // Try tofind the image and if it found - break
    // Processing each image with iterator img
    for img in &images {
        // Processing all repo_tags with iterator i
        for i in img.repo_tags.clone () {
            // Variable "i" contains the name of the image
            // If it contains language specific work then OK and break the cicle
            if i.contains (&format!("{}", &lang.to_string())) {
                println! ("Image for {:?} is found", lang);
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
    // Or returns error message if can't connect to the Docker

}
