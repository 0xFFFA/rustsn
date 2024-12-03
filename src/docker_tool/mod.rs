/*
This file contains the definitions for docker_tool module
*/

// Add these libs to work with a bollard::Docker
use bollard::Docker;
use bollard::image::ListImagesOptions;
use bollard::image::CreateImageOptions;
use bollard::container::{CreateContainerOptions, Config as ContainerConfig, StartContainerOptions};
use bollard::service::HostConfig; 

// Add these libs to work with bollard::Docker specific functions
use std::default::Default;
use futures::StreamExt;
use tokio::runtime::Runtime;

// Add this crate to work with Lang and VERBOSE
use crate::{Lang, VERBOSE};

// Define entity which contains the type of the environment - host or docker
#[allow(non_camel_case_types)]
pub enum EnvironmentType {
    host,
    docker
}

/*
Define the modules of docker_tool
One function - one file

check_docker_images.rs - checks the list of language specific images, this function uses by check_docker function
check_docker.rs - checks the Docker status
create_image_and_container.rs - OLD function, splits to two separate functions. creates language specific image and container
run_container.rs - runs the container
stop_container.rs - stops the container

!!! WARNING !!!
Necessary changes:
- split function create_image_and_container into two functions DONE
  - create_image DONE
  - create_container DINE
- add new function remove_container DONE
- add prompt to function create_image to download image from the Docker Hub DONE

- add functionality to check and stop container then the app starts
- remove from code fn create_image_and_container

*/
pub mod check_docker_images;
pub mod check_docker;
pub mod create_image_and_container;
pub mod run_container;
pub mod stop_container;
pub mod remove_container;
pub mod create_image;
pub mod create_container;

pub use check_docker_images::check_docker_images;
pub use check_docker::check_docker;
pub use create_image_and_container::create_image_and_container;
pub use run_container::run_container;
pub use stop_container::stop_container;
pub use remove_container::remove_container;
pub use create_image::create_image;
pub use create_container::create_container;


/*

There is some idea to release this module like an object
Structure this has entity image and vector of entities for containers
And define the functions for the structure
check image, check container, create image, create container, run container, stop container,
remove container, remove image
Probably it could be much consisten then this separate functions
But it means it is neccesary to change a lot of code, because
it would be necessary to transfer the variable this contain a value of structcure 
from function to function
*/